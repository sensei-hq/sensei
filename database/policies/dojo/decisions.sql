-- RLS for dojo.decisions (#236 step 3; access matrix "Knowledge").
--
-- A decision is the non-repudiation trail of a maintainer's verdict, so a person
-- records only their OWN decision, by hand (not `automated`), about an artifact
-- — and a triage row — of the tenant it is filed in. The tenant's maintainers
-- and admins read the trail. Automated decisions stay with the system.
-- Covered by database/tests/dojo/knowledge_access.sql.
alter table dojo.decisions enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists decisions_service_only on dojo.decisions;

grant select on dojo.decisions to authenticated;
grant insert (tenant_id, artifact_id, triage_id, maintainer_id, status, reason,
              distribution_scope, automated)
    on dojo.decisions to authenticated;

drop policy if exists decisions_read_maintainer on dojo.decisions;
create policy decisions_read_maintainer on dojo.decisions
    for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'maintainer'));

drop policy if exists decisions_record_own on dojo.decisions;
create policy decisions_record_own on dojo.decisions
    for insert to authenticated
    with check (
        dojo.tenant_role_at_least(tenant_id, 'maintainer')
        and maintainer_id = dojo.current_principal_id()
        and not automated
        and exists (select 1 from dojo.artifacts a
                     where a.id = decisions.artifact_id and a.tenant_id = decisions.tenant_id)
        and (triage_id is null
             or exists (select 1 from dojo.triage_queue q
                         where q.id = decisions.triage_id and q.tenant_id = decisions.tenant_id))
    );
