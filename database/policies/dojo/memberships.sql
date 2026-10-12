-- RLS for dojo.memberships (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- A person reads their own memberships — every dōjō they belong to; a tenant
-- admin reads the tenant's, and sets a member's role. Nothing else is anyone's
-- to change from their session: joining is by invitation and provisioning, both
-- the system's.
--
-- The SELECT grant is by column. device_key and device_token_hash are the daemon
-- plane's credential lookup, read by the server alone, so no session reads them
-- — not even the member's own.
-- Covered by database/tests/dojo/tenancy_access.sql.
alter table dojo.memberships enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists memberships_service_only on dojo.memberships;

grant select (id, tenant_id, user_id, role, kind, org_slugs, authenticated_via, attribution_default,
              sync_status, last_heartbeat_at, disabled_at, created_at, updated_at)
    on dojo.memberships to authenticated;
grant update (role, updated_at) on dojo.memberships to authenticated;

drop policy if exists memberships_read on dojo.memberships;
create policy memberships_read on dojo.memberships
    for select to authenticated
    using (user_id = dojo.current_principal_id() or dojo.tenant_role_at_least(tenant_id, 'admin'));

drop policy if exists memberships_role_admin on dojo.memberships;
create policy memberships_role_admin on dojo.memberships
    for update to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'))
    with check (dojo.tenant_role_at_least(tenant_id, 'admin'));
