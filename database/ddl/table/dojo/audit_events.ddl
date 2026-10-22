set search_path to dojo, extensions;

create table if not exists dojo.audit_events (
  id            bigserial   primary key
, tenant_id     uuid        not null references dojo.tenants(id)
, ts            timestamptz not null default now()
, actor_id      uuid
, action        text        not null
, target        text
, detail        jsonb       not null default '{}'
);

create index if not exists audit_events_tenant_idx on dojo.audit_events(tenant_id);
create index if not exists audit_events_actor_idx on dojo.audit_events(tenant_id, actor_id);

comment on table dojo.audit_events is
'Append-only non-repudiation audit, stamped by the server on every admin action
and rule federation (publish / retract). Serves the admin console audit log.
Distinct from dojo.events (operational loop log): this one is the governance
trail and its columns are a subset covered by the universal strip so it never
leaks source references.';

comment on column dojo.audit_events.actor_id
     is 'user_id of the person who performed the action.';
comment on column dojo.audit_events.action
     is 'Admin/governance verb (e.g. member_added, role_changed, policy_edited, publish, retract).';
comment on column dojo.audit_events.target
     is 'Human-readable target of the action (member id, policy key, rule).';

-- RLS: the access matrix's cells for this table. The policies call the access
-- predicates and so live in policies/dojo/audit_events.sql (see policies/README.md).
alter table dojo.audit_events enable row level security;
