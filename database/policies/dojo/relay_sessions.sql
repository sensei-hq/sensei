-- RLS for dojo.relay_sessions.
--
-- Lives here, not in ddl/table/dojo/relay_sessions.ddl, because this policy CALLS a
-- function and `dbd apply` creates every table before any function. Inline, it
-- failed the deploy outright:
--   function dojo.owns_membership(...) does not exist
--
-- `dbd policies` runs after all entities exist, which is the only ordering in
-- which a function-dependent policy can be created. Policies that call no
-- function (auth.uid() alone) stay with their table — see policies/README.md.

-- Row-Level Security (P4.1) — client-direct Supabase Realtime read path.
-- The phone subscribes Supabase Realtime with its own JWT (role `authenticated`),
-- so it can only ever SELECT/subscribe rows RLS lets it see. The Worker keeps its
-- `service_role` write path, which BYPASSES RLS (rolbypassrls) — daemon→Worker→DB
-- writes are unaffected. SELECT-only policy: the phone only reads; every write
-- stays on the service-role path.
--
-- P4 model = own-rows-only: a user sees a run only if it is THEIR run — derived
-- from membership_id via dojo.owns_membership() (NOT a denormalized user_id copy,
-- which goes stale on re-ownership — WS-0 Rule A). Team-wide visibility (all
-- members of the run's tenant see it) is a deliberate P6 extension — NOT P4.
--
-- Idempotent: the pre-release deploy re-applies every DDL file declaratively on
-- each deploy (dbd `Current`/`Fresh` strategy re-runs ApplyEntity → re-executes
-- this file), and `create policy` is not idempotent — so drop-if-exists first.
-- `enable row level security` is idempotent (no error if already enabled).
alter table dojo.relay_sessions enable row level security;

-- The client-direct read path connects as role `authenticated`. RLS filters ROWS
-- but a table-level GRANT is still required for the role to touch the table at all
-- (without it: "permission denied for table", RLS never even evaluated). SELECT
-- only — writes stay on the service_role path. The dojo schema already grants
-- USAGE to authenticated (exposed schema); this adds the row-read privilege that
-- the own-rows policy then narrows. Idempotent (re-grant is a no-op).
grant select on dojo.relay_sessions to authenticated;

drop policy if exists relay_sessions_select_own on dojo.relay_sessions;
create policy relay_sessions_select_own
    on dojo.relay_sessions
    for select
    to authenticated
    using (dojo.owns_membership(membership_id));

-- Realtime publication membership — declared HERE, with the table's other
-- exposure config, not in supabase/migrations/.
--
-- It used to live in a Supabase migration, on the reasoning that
-- `supabase_realtime` is a Supabase-owned object. The flaw is lifecycle:
-- DROPPING a table removes it from a publication, and RE-CREATING it does not
-- put it back. So a `dbd reset` + redeploy of the dojo scope left every relay
-- table out of the publication — Realtime silently delivering nothing, no error
-- anywhere — until somebody remembered to re-run the migration. Verified.
--
-- dbd already owns this table's grants and RLS; publication membership is the
-- same category of thing (who may see this table, through which transport) and
-- has to share the table's lifecycle to be correct.
--
-- Why a policy file and not the table DDL: dbd's SQL parser cannot read a
-- `do $$ … $$` block, and a file it cannot parse is dropped from the entity set
-- SILENTLY. Policy files are executed as raw SQL rather than parsed as entities,
-- and they run after every entity exists — which is also the only correct
-- ordering, since the table must be there before it can join a publication.
--
-- Guarded twice: skip when the publication does not exist (a plain Postgres, so
-- this file stays harmless off-Supabase), and skip when the table is already a
-- member (`alter publication … add table` ERRORS on a duplicate, which would
-- fail the deploy on the second run).
do $$
begin
  if exists (select 1 from pg_publication where pubname = 'supabase_realtime')
     and not exists (
       select 1 from pg_publication_tables
        where pubname = 'supabase_realtime'
          and schemaname = 'dojo'
          and tablename = 'relay_sessions')
  then
    alter publication supabase_realtime add table dojo.relay_sessions;
  end if;
end $$;
