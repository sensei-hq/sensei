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

-- Scope guard. `dbd policies` applies every file under policies/ regardless of
-- --scope, so this file also runs against the daemon plane, where the `dojo`
-- schema does not exist. Ungated it reported "FAILED … schema dojo does not
-- exist" on every deploy there — an expected condition dressed as an error,
-- which is how real failures stop being read. Logged as a dbd gap in
-- docs/backlog.md; guarded here so both planes deploy clean.
do $$
begin
  if to_regclass('dojo.relay_sessions') is null then
    return;
  end if;

  -- RLS filters ROWS; a table-level GRANT is still what lets the role touch
  -- the table at all (without it: "permission denied", RLS never evaluated).
  execute $stmt$alter table dojo.relay_sessions enable row level security$stmt$;
  execute $stmt$grant select on dojo.relay_sessions to authenticated$stmt$;
  execute $stmt$drop policy if exists relay_sessions_select_own on dojo.relay_sessions$stmt$;
  execute $stmt$create policy relay_sessions_select_own on dojo.relay_sessions
     for select to authenticated
     using (dojo.owns_membership(membership_id))$stmt$;
end $$;
