-- RLS for dojo.push_subscriptions (#236 step 3b; access matrix "Relay").
--
-- A device registration is the person's own, filed under one of their own
-- memberships: the endpoint is where the dōjō sends that membership's gates, so
-- a subscription under someone else's membership would read their relay. The
-- push send reads these as service_role, which bypasses RLS.
--
-- user_id is never updatable: a registration does not change hands.
alter table dojo.push_subscriptions enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists push_subscriptions_service_only on dojo.push_subscriptions;

grant select on dojo.push_subscriptions to authenticated;
grant insert (user_id, membership_id, platform, web_push, enabled, last_seen)
    on dojo.push_subscriptions to authenticated;
grant update (membership_id, platform, web_push, enabled, last_seen, updated_at)
    on dojo.push_subscriptions to authenticated;

drop policy if exists push_subscriptions_own on dojo.push_subscriptions;
create policy push_subscriptions_own on dojo.push_subscriptions
    for all to authenticated
    using (user_id = dojo.current_principal_id())
    with check (user_id = dojo.current_principal_id() and dojo.owns_membership(membership_id));
