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
-- Covering index for the artifact_id FK (nullable) — avoids a seq-scan on
-- dojo.artifacts deletes (Supabase unindexed_foreign_keys).
create index if not exists incidents_artifact_idx on dojo.incidents(artifact_id) where artifact_id is not null;

comment on table dojo.incidents is
'A confidentiality incident (a near-leak on client work) tracked by the
lead. Open incidents are rows where resolved_at is null. Carries
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

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.incidents enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists incidents_service_only on dojo.incidents;
create policy incidents_service_only on dojo.incidents
    for all to authenticated, anon
    using (false) with check (false);
