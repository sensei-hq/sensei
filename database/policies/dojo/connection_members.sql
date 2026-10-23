-- RLS for dojo.connection_members (#308 R5c): the system's alone — the member
-- read (service_role, through dojo.reconcile_unit_members) writes it and a
-- sign-in reads it (dojo.units_listing_member); no session reads or writes a
-- member set. Covered by database/tests/dojo/connection_member_sets.sql.
alter table dojo.connection_members enable row level security;

revoke all on dojo.connection_members from authenticated, anon;

drop policy if exists connection_members_service_only on dojo.connection_members;
create policy connection_members_service_only on dojo.connection_members
    for all to authenticated, anon using (false) with check (false);
