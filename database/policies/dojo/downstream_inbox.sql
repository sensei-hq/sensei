-- RLS for dojo.downstream_inbox (#236 step 3; entity model §3 — personal work).
--
-- What reached a person through their memberships is theirs: they read it and
-- mark it adopted (state, acted_at). Nobody else reads it, a tenant admin
-- included, and a row never changes membership or artifact. Delivery is the
-- system's.
-- Covered by database/tests/dojo/contributions_access.sql.
alter table dojo.downstream_inbox enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists downstream_inbox_service_only on dojo.downstream_inbox;

grant select on dojo.downstream_inbox to authenticated;
grant update (state, acted_at, updated_at) on dojo.downstream_inbox to authenticated;

drop policy if exists downstream_inbox_own on dojo.downstream_inbox;
create policy downstream_inbox_own on dojo.downstream_inbox
    for all to authenticated
    using (dojo.owns_membership(membership_id))
    with check (dojo.owns_membership(membership_id));
