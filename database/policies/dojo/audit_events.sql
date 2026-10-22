-- RLS for dojo.audit_events (#236 step 3; access matrix "Audit").
--
-- A tenant admin reads the whole trail; nobody below admin reads it (the lead's
-- confidentiality ledger left with the Clients section, #307 R7, D34). Nobody
-- writes from their own session:
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
    using (dojo.tenant_role_at_least(tenant_id, 'admin'));
