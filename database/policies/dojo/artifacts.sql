-- RLS for dojo.artifacts (#236 step 3; access matrix "Knowledge").
--
-- A member of the artifact's tenant reads it once published, and always reads
-- their own submissions; a maintainer or admin reads the tenant's submissions
-- before triage. Writes stay on the device plane (submissions) and the system
-- (clustering), as the service role, until step 4.
-- Covered by database/tests/dojo/knowledge_access.sql.
alter table dojo.artifacts enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists artifacts_service_only on dojo.artifacts;

grant select on dojo.artifacts to authenticated;

drop policy if exists artifacts_read on dojo.artifacts;
create policy artifacts_read on dojo.artifacts
    for select to authenticated
    using (
        dojo.is_member(tenant_id)
        and (
            status = 'published'
            or contributed_by = dojo.current_principal_id()
            or dojo.tenant_role_at_least(tenant_id, 'maintainer')
        )
    );
