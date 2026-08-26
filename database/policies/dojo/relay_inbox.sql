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
--
-- Plain SQL, no scope guard: since dbd 0.12.0 `policies` honours --scope and
-- skips a file whose table is out of scope ("Skipped … outside scope 'default'")
-- rather than failing it. Earlier versions applied every file regardless, which
-- forced each statement through a `do $$ … execute … $$` block just to no-op on
-- the daemon plane.

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
