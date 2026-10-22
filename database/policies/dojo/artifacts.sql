-- RLS for dojo.artifacts (#236 step 3; access matrix "Knowledge").
--
-- A member of the artifact's tenant reads it once published, and always reads
-- their own submissions; a maintainer or admin reads the tenant's submissions
-- before triage. A member contributes a submission as themselves (#236 step 4);
-- publishing, approving and the counters stay the system's (clustering).
-- Covered by database/tests/dojo/knowledge_access.sql and artifacts_publish.sql.
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

-- Who contributes (#236 step 4, POST artifacts): an active member, as
-- themselves, into triage. `status`, `approved_by`, `published_at` and the
-- counters are not granted, so a submission lands at the column default
-- ('submitted') and only promoteCluster or a maintainer moves it on.
grant insert (tenant_id, kind, title, body, payload, scope, signature, attribution, contributed_by)
   on dojo.artifacts to authenticated;
-- `seq` defaults to nextval: the cursor the daemon's pull reads from.
grant usage on sequence dojo.artifacts_seq to authenticated;

drop policy if exists artifacts_contribute on dojo.artifacts;
create policy artifacts_contribute on dojo.artifacts
    for insert to authenticated
    with check (
        dojo.is_member(tenant_id)
        and contributed_by = dojo.current_principal_id()
    );
