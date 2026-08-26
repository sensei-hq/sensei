-- RLS for dojo.relay_inbox.
--
-- Lives here, not in ddl/table/dojo/relay_inbox.ddl, because this policy CALLS a
-- function and `dbd apply` creates every table before any function. Inline, it
-- failed the deploy outright:
--   function dojo.owns_membership(...) does not exist
--
-- `dbd policies` runs after all entities exist, which is the only ordering in
-- which a function-dependent policy can be created. Policies that call no
-- function (auth.uid() alone) stay with their table — see policies/README.md.

-- Row-Level Security (P4.1) — see the note on dojo.relay_sessions. Own-rows-only:
-- a user reads/subscribes only their own inbox rows — ownership derived from
-- membership_id via dojo.owns_membership() (no stale user_id copy — WS-0 Rule A).
-- The Worker's service_role writes bypass RLS. SELECT-only; team-wide visibility is P6.
-- Idempotent (drop-if-exists) because the deploy re-applies this file each time.
alter table dojo.relay_inbox enable row level security;

-- Table-level SELECT grant for the `authenticated` read path (RLS filters rows;
-- the grant lets the role touch the table). See the note on dojo.relay_sessions.
grant select on dojo.relay_inbox to authenticated;

drop policy if exists relay_inbox_select_own on dojo.relay_inbox;
create policy relay_inbox_select_own
    on dojo.relay_inbox
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
          and tablename = 'relay_inbox')
  then
    alter publication supabase_realtime add table dojo.relay_inbox;
  end if;
end $$;
