set search_path to dojo, extensions;

create table if not exists dojo.engagements (
  id               uuid                    primary key default gen_random_uuid()
, tenant_id        uuid                    not null references dojo.tenants(id)
, client_tenant_id uuid                    references dojo.tenants(id)
, client_name      text                    not null
, description      text
, project_bindings jsonb                   not null default '[]'
, policy_overrides jsonb                   not null default '{}'
, status           dojo.engagement_status  not null default 'active'
, starts_on        date
, ends_on          date
, created_at       timestamptz             not null default now()
, updated_at       timestamptz             not null default now()
);

create index if not exists engagements_tenant_idx on dojo.engagements(tenant_id);
-- Covering index for the client_tenant_id FK (nullable) — avoids a seq-scan when a
-- referenced dojo.tenants row is deleted (Supabase unindexed_foreign_keys).
create index if not exists engagements_client_tenant_idx on dojo.engagements(client_tenant_id) where client_tenant_id is not null;

comment on table dojo.engagements is
'A registered client engagement, owned by a lead. Binds client work to
projects so it routes correctly. Every artifact shared under an engagement is
source-dereferenced on publish (the always-on invariant) — the lead cannot
per-item override the strip; they audit that it held.';

comment on column dojo.engagements.client_name
     is 'Client / engagement display name (always set).';
comment on column dojo.engagements.client_tenant_id
     is 'Optional FK to the client''s own dojo.tenants row when the client is itself a known tenant; null otherwise.';
comment on column dojo.engagements.project_bindings
     is 'Daemon-side project references routed to this engagement: [{project_id, name}]. Advisory (projects live in the daemon DB, Fork 1).';
comment on column dojo.engagements.policy_overrides
     is 'Engagement-specific overrides on the tenant policy (retention, confidentiality pack, etc.).';
comment on column dojo.engagements.status
     is 'active (in force, routes + dereferences) or ended (closed, retained for audit).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.engagements enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists engagements_service_only on dojo.engagements;
create policy engagements_service_only on dojo.engagements
    for all to authenticated, anon
    using (false) with check (false);
