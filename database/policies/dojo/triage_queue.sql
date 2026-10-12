-- RLS for dojo.triage_queue (#236 step 3; access matrix "Knowledge").
--
-- The triage queue is the tenant's maintainers' (and admins'): they read it and
-- move a row's state when they decide it. Nothing else about a row is theirs to
-- change, hence the column grant. Queueing stays with the system.
-- Covered by database/tests/dojo/knowledge_access.sql.
alter table dojo.triage_queue enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists triage_queue_service_only on dojo.triage_queue;

grant select on dojo.triage_queue to authenticated;
grant update (state, updated_at) on dojo.triage_queue to authenticated;

drop policy if exists triage_queue_maintainer on dojo.triage_queue;
create policy triage_queue_maintainer on dojo.triage_queue
    for all to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'maintainer'))
    with check (dojo.tenant_role_at_least(tenant_id, 'maintainer'));
