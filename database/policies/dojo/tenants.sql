-- RLS for dojo.tenants (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- A member reads the identity of each dōjō they belong to — name, key, slug,
-- origin, url — which is what lists and labels their dōjōs. The tenant's
-- settings and claim are not part of that identity and no session reads them;
-- creating and claiming a tenant is provisioning's (the system's).
-- Covered by database/tests/dojo/contributions_access.sql.
alter table dojo.tenants enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists tenants_service_only on dojo.tenants;

grant select (id, key, origin, slug, dojo, scope, name, dojo_url, self_hosted, created_at, updated_at)
    on dojo.tenants to authenticated;

drop policy if exists tenants_read_member on dojo.tenants;
create policy tenants_read_member on dojo.tenants
    for select to authenticated
    using (dojo.is_member(id));
