-- RLS for dojo.identities (#236 step 3; access matrix "Tenancy, identity, billing").
--
-- An identity is the PERSON's and spans tenants. They read their own; a tenant
-- admin reads the identities of the tenant's ACTIVE members and of its dōjō
-- viewers (#307 D8). Nobody writes one from their session: binding a forge
-- account needs that forge's proof, which is provisioning's (the system's) job.
-- Covered by database/tests/dojo/tenancy_access.sql and project_viewers.sql.
alter table dojo.identities enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists identities_service_only on dojo.identities;

grant select on dojo.identities to authenticated;

drop policy if exists identities_read on dojo.identities;
create policy identities_read on dojo.identities
    for select to authenticated
    using (
        principal_id = dojo.current_principal_id()
        or exists (select 1 from dojo.memberships m
                    where m.user_id = identities.principal_id
                      and m.disabled_at is null
                      and dojo.tenant_role_at_least(m.tenant_id, 'admin'))
        -- A dōjō viewer (#307 D8) is not a member; the dōjō's admins still name them.
        or exists (select 1 from dojo.dojo_viewers v
                    where v.principal_id = identities.principal_id
                      and dojo.tenant_role_at_least(v.tenant_id, 'admin'))
    );
