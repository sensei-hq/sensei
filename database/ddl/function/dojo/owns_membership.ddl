set search_path to dojo, extensions;

-- Ownership predicate for the user-work relay_* tables (WS-0 Rule A). Derives
-- ownership from membership_id via the memberships FK instead of a denormalized
-- user_id copy that goes stale on re-ownership (re-pairing / ownership transfer /
-- the F6 verify re-own). One membership → exactly one (user_id, tenant_id), so
-- the membership is the single, live ownership reference; RLS on relay_sessions /
-- relay_inbox / relay_segments all call this. See
-- docs/design/2026-07-27-dojo-relay-rls-membership-function.md.
--
-- STABLE            → the planner evaluates it once per statement (initplan), not
--                     per row.
-- SECURITY DEFINER  → reads dojo.memberships regardless of the caller's grants;
--                     search_path is pinned to dojo so the lookup can't be
--                     hijacked by a caller-set search_path.
-- (select auth.uid()) → the Supabase-recommended form so auth.uid() is folded to
--                     an initplan constant.
create or replace function dojo.owns_membership(mid uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo
as $$
    select exists (
        select 1
        from dojo.memberships m
        where m.id = mid
          and m.user_id = (select auth.uid())
    );
$$;

-- The relay RLS policies (relay_sessions/inbox/segments) call this predicate as the
-- querying role, so `authenticated` MUST have EXECUTE for those policies to evaluate.
-- Revoke the public default, grant to authenticated.
-- NOTE (advisor "SECURITY DEFINER RPC callable by authenticated"): INTENTIONAL and safe —
-- the EXECUTE grant is required for RLS, and a direct RPC call only reveals whether the
-- CALLER owns the passed membership id (their own fact), never another user's data. Keep.
revoke all on function dojo.owns_membership(uuid) from public;
grant execute on function dojo.owns_membership(uuid) to authenticated;
