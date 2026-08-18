set search_path to dojo, sensei, extensions;

-- The live billable seat count per tenant: unique users ACTIVELY seated on a
-- private project. A user on several private projects counts once (deduped on
-- user_id); users only on public projects, and closed seats (ended_at set), are
-- excluded. This is the source of truth the billing route reads and caches into
-- dojo.billing_accounts.seats_used.
--
-- "Actively using sensei" here means holding an OPEN seat (ended_at IS NULL) — a
-- seat opened on active use and closed on removal/inactivity. A route may tighten
-- this to last_active_at within the billing window; the raw open-seat count is the
-- defensible floor.
-- SECURITY: `security_invoker = on` — runs as the calling role, not the owner (Supabase lint
-- security_definer_view). Read only server-side as service_role (granted SELECT on dojo.seats +
-- sensei.namespaces; service_role bypasses RLS); NOT granted to any client role, so billing
-- seat counts are never exposed via PostgREST.
create or replace view dojo.tenant_seat_usage
with (security_invoker = on)
as
select s.tenant_id
     , count(distinct s.user_id) as seats_used
  from dojo.seats s
  join sensei.namespaces n on n.id = s.namespace_id
 where s.ended_at is null
   and n.visibility = 'private'
 group by s.tenant_id;

comment on view dojo.tenant_seat_usage is
'Live billable seat count per tenant: COUNT(DISTINCT user_id) over active
(ended_at IS NULL) dojo.seats whose project namespace is private. Deduped per
user across projects; public-only users and closed seats excluded. The billing
route reads this and caches it into dojo.billing_accounts.seats_used.';
