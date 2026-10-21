-- RLS for dojo.invites (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- A tenant admin issues invites, in their own name, and reads the tenant's.
-- Accepting one is the system's (the joining person has no membership yet; the
-- token is the authority), so no session updates or deletes an invite.
-- Covered by database/tests/dojo/invites_access.sql.
alter table dojo.invites enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists invites_service_only on dojo.invites;

-- By column: never `token_hash`. The token is a bearer, redeemed only through the
-- accept route; the admin who issues an invite is handed the token the server minted.
grant select (id, tenant_id, email, role, kind, access, invited_by, expires_at, accepted_at, created_at)
    on dojo.invites to authenticated;
grant insert (tenant_id, email, role, kind, access, token_hash, invited_by, expires_at) on dojo.invites to authenticated;

drop policy if exists invites_read_admin on dojo.invites;
create policy invites_read_admin on dojo.invites
    for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));

drop policy if exists invites_issue_admin on dojo.invites;
create policy invites_issue_admin on dojo.invites
    for insert to authenticated
    with check (dojo.tenant_role_at_least(tenant_id, 'admin') and invited_by = dojo.current_principal_id());
