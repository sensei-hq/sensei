set search_path to dojo, extensions;

-- The client confidentiality ledger: the audit events dojo.is_confidentiality_action
-- names, and nothing else. `security_invoker = on`, so dojo.audit_events' policies
-- decide the rows — a tenant lead or admin reads their own tenant's ledger.
create or replace view dojo.confidentiality_ledger
with (security_invoker = on)
as
select id, tenant_id, ts, action, target, detail, engagement_id
  from dojo.audit_events
 where dojo.is_confidentiality_action(action);

grant select on dojo.confidentiality_ledger to authenticated;

comment on view dojo.confidentiality_ledger is
'Client confidentiality ledger (publish / contained / held audit events), read
as the caller under dojo.audit_events'' RLS (#236).';
