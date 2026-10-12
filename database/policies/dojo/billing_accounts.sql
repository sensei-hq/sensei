-- RLS for dojo.billing_accounts (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- A tenant admin reads the tenant's billing account. Nobody writes it from a
-- session: the account follows the payment provider, and seats_used is a cache
-- derived from the seats — a money-facing number the system refreshes, never a person.
-- Covered by database/tests/dojo/billing_access.sql.
alter table dojo.billing_accounts enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists billing_accounts_service_only on dojo.billing_accounts;

grant select on dojo.billing_accounts to authenticated;

drop policy if exists billing_accounts_read_admin on dojo.billing_accounts;
create policy billing_accounts_read_admin on dojo.billing_accounts
    for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));
