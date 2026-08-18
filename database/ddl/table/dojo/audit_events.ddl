set search_path to dojo, extensions;

create table if not exists dojo.audit_events (
  id            bigserial   primary key
, tenant_id     uuid        not null references dojo.tenants(id)
, ts            timestamptz not null default now()
, actor_id      uuid
, engagement_id uuid        references dojo.engagements(id)
, action        text        not null
, target        text
, detail        jsonb       not null default '{}'
);

create index if not exists audit_events_tenant_idx on dojo.audit_events(tenant_id);
create index if not exists audit_events_actor_idx on dojo.audit_events(tenant_id, actor_id);
create index if not exists audit_events_engagement_idx on dojo.audit_events(engagement_id);

comment on table dojo.audit_events is
'Append-only compliance / non-repudiation audit, stamped by the auth middleware
on every admin and lead action. Serves the admin console audit log and
the lead compliance export (filterable by engagement). Distinct from
dojo.events (operational loop log): this one is the confidentiality/governance
trail and its columns are a subset covered by the universal strip so exports
never leak source references.';

comment on column dojo.audit_events.actor_id
     is 'user_id of the admin/lead who performed the action.';
comment on column dojo.audit_events.engagement_id
     is 'The engagement in scope, for engagement-filtered compliance exports.';
comment on column dojo.audit_events.action
     is 'Admin/governance verb (e.g. member_added, role_changed, policy_edited, engagement_created, incident_opened).';
comment on column dojo.audit_events.target
     is 'Human-readable target of the action (member id, policy key, engagement name).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.audit_events enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists audit_events_service_only on dojo.audit_events;
create policy audit_events_service_only on dojo.audit_events
    for all to authenticated, anon
    using (false) with check (false);
