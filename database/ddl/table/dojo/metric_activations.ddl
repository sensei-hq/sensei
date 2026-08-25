set search_path to dojo, extensions;

-- Which catalogue metrics a tenant has switched on.
--
-- The one thing a tenant controls about the catalogue. Disabling a metric both
-- hides it and STOPS ITS COMPUTATION — the planner enqueues no task for a
-- deactivated metric — so this is a cost lever, not a display preference.
--
-- Absence means enabled. A tenant that has never touched this table gets the
-- whole catalogue, which is the sane default and avoids having to seed a row per
-- (tenant × metric) on tenant creation.
create table if not exists metric_activations (
  tenant_id   uuid        not null references dojo.tenants(id) on delete cascade
, metric_id   uuid        not null references dojo.metrics(id) on delete cascade
, enabled     boolean     not null default true
, updated_at  timestamptz not null default now()
, primary key (tenant_id, metric_id)
);

comment on table metric_activations is
'Per-tenant on/off for catalogue metrics. Absence = enabled, so a new tenant
needs no seeding. Disabling skips the computation, not just the display.';

alter table metric_activations enable row level security;
drop policy if exists metric_activations_service_only on metric_activations;
create policy metric_activations_service_only on metric_activations
    for all to authenticated, anon using (false) with check (false);
