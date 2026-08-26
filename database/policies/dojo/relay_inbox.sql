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

-- Scope guard. `dbd policies` applies every file under policies/ regardless of
-- --scope, so this file also runs against the daemon plane, where the `dojo`
-- schema does not exist. Ungated it reported "FAILED … schema dojo does not
-- exist" on every deploy there — an expected condition dressed as an error,
-- which is how real failures stop being read. Logged as a dbd gap in
-- docs/backlog.md; guarded here so both planes deploy clean.
do $$
begin
  if to_regclass('dojo.relay_inbox') is null then
    return;
  end if;

  -- Same pairing as relay_sessions: enable RLS, grant the row-read privilege
  -- the own-rows policy then narrows.
  execute $stmt$alter table dojo.relay_inbox enable row level security$stmt$;
  execute $stmt$grant select on dojo.relay_inbox to authenticated$stmt$;
  execute $stmt$drop policy if exists relay_inbox_select_own on dojo.relay_inbox$stmt$;
  execute $stmt$create policy relay_inbox_select_own on dojo.relay_inbox
     for select to authenticated
     using (dojo.owns_membership(membership_id))$stmt$;
end $$;
