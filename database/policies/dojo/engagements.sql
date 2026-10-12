-- RLS for dojo.engagements (#236 step 3; access matrix "Client work and compliance").
--
-- Engagements name clients: a tenant's leads and admins read, open, change and
-- close them; nobody below lead sees one. An engagement never changes tenant and
-- the client it names is set when it opens, so neither is in the UPDATE grant.
-- Covered by database/tests/dojo/engagements_access.sql.
alter table dojo.engagements enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists engagements_service_only on dojo.engagements;

grant select, delete on dojo.engagements to authenticated;
grant insert (tenant_id, client_tenant_id, client_name, description, project_bindings,
              policy_overrides, status, starts_on, ends_on)
    on dojo.engagements to authenticated;
grant update (description, status, starts_on, ends_on, policy_overrides, project_bindings, updated_at)
    on dojo.engagements to authenticated;

drop policy if exists engagements_lead on dojo.engagements;
create policy engagements_lead on dojo.engagements
    for all to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'lead'))
    with check (dojo.tenant_role_at_least(tenant_id, 'lead'));
