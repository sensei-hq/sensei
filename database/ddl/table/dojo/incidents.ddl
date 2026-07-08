set search_path to dojo, extensions;

create table if not exists dojo.incidents (
  id            uuid                   primary key default gen_random_uuid()
, tenant_id     uuid                   not null references dojo.tenants(id)
, engagement_id uuid                   references dojo.engagements(id)
, artifact_id   uuid                   references dojo.artifacts(id)
, title         text                   not null
, description   text
, severity      dojo.incident_severity not null default 'medium'
, status        dojo.incident_status   not null default 'open'
, owner_id      uuid
, sla_due_at    timestamptz
, resolution    text
, opened_at     timestamptz            not null default now()
, resolved_at   timestamptz
, created_at    timestamptz            not null default now()
, updated_at    timestamptz            not null default now()
);

create index if not exists incidents_tenant_idx on dojo.incidents(tenant_id);
create index if not exists incidents_engagement_idx on dojo.incidents(engagement_id);
create index if not exists incidents_open_idx on dojo.incidents(tenant_id) where resolved_at is null;

comment on table dojo.incidents is
'A confidentiality incident (a near-leak on client work) tracked by the
client-lead. Open incidents are rows where resolved_at is null. Carries
severity, owner, and an SLA for alerting.';

comment on column dojo.incidents.engagement_id
     is 'The engagement the incident relates to, when applicable.';
comment on column dojo.incidents.artifact_id
     is 'The artifact involved in the near-leak, when identifiable.';
comment on column dojo.incidents.severity
     is 'low | medium | high | critical (drives SLA + alerting).';
comment on column dojo.incidents.status
     is 'open | investigating | resolved.';
comment on column dojo.incidents.owner_id
     is 'user_id of the incident owner.';
comment on column dojo.incidents.sla_due_at
     is 'When the incident breaches SLA if still open (alert threshold).';
