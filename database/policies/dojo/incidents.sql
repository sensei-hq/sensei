-- RLS for dojo.incidents (#236 step 3; access matrix "Client work and compliance").
--
-- A tenant's leads and admins read, open, change and delete its incidents;
-- nobody below lead sees one. Every reference an incident carries must name an
-- object of its own tenant — its engagement, its artifact, and its owner, an
-- active member (dojo.principal_is_member). An incident never changes tenant.
-- Covered by database/tests/dojo/incidents_access.sql.
alter table dojo.incidents enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists incidents_service_only on dojo.incidents;

grant select, delete on dojo.incidents to authenticated;
grant insert (tenant_id, engagement_id, artifact_id, title, description, severity, status,
              owner_id, sla_due_at)
    on dojo.incidents to authenticated;
grant update (severity, status, owner_id, sla_due_at, resolution, resolved_at, updated_at)
    on dojo.incidents to authenticated;

drop policy if exists incidents_lead on dojo.incidents;
create policy incidents_lead on dojo.incidents
    for all to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'lead'))
    with check (
        dojo.tenant_role_at_least(tenant_id, 'lead')
        and (engagement_id is null
             or exists (select 1 from dojo.engagements e
                         where e.id = incidents.engagement_id and e.tenant_id = incidents.tenant_id))
        and (artifact_id is null
             or exists (select 1 from dojo.artifacts a
                         where a.id = incidents.artifact_id and a.tenant_id = incidents.tenant_id))
        and (owner_id is null or dojo.principal_is_member(tenant_id, owner_id))
    );
