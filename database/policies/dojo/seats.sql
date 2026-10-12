-- RLS for dojo.seats (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- A tenant admin reads the tenant's seats. Nobody writes one from a
-- session: seats open and close with relay activity (the system's).
-- Covered by database/tests/dojo/billing_access.sql.
alter table dojo.seats enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists seats_service_only on dojo.seats;

grant select on dojo.seats to authenticated;

drop policy if exists seats_read_admin on dojo.seats;
create policy seats_read_admin on dojo.seats
    for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));
