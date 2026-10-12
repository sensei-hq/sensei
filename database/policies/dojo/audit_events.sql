-- RLS for dojo.audit_events (#236 step 3; access matrix "Client work and compliance").
--
-- A tenant admin reads the whole trail; a tenant lead reads its confidentiality
-- ledger (dojo.is_confidentiality_action). Nobody writes from their own session:
-- an audit row is the system's record of what a person did, written by the
-- server as the service role — so authenticated gets SELECT and nothing else.
-- Covered by database/tests/dojo/audit_access.sql.
alter table dojo.audit_events enable row level security;

-- Replaces the deny-all from when the table was server-only.
drop policy if exists audit_events_service_only on dojo.audit_events;

grant select on dojo.audit_events to authenticated;

drop policy if exists audit_events_read on dojo.audit_events;
create policy audit_events_read on dojo.audit_events
    for select to authenticated
    using (
        dojo.tenant_role_at_least(tenant_id, 'admin')
        or (dojo.tenant_role_at_least(tenant_id, 'lead') and dojo.is_confidentiality_action(action))
    );
