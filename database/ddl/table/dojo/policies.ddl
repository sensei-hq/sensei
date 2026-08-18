set search_path to dojo, extensions;

create table if not exists dojo.policies (
  id                  uuid                  primary key default gen_random_uuid()
, tenant_id           uuid                  not null references dojo.tenants(id)
, scope_key           text                  not null
, attribution_default dojo.attribution_mode not null default 'named'
, confidentiality     jsonb                 not null default '{}'
, retention_days      integer
, created_at          timestamptz           not null default now()
, updated_at          timestamptz           not null default now()
, constraint policies_tenant_scope_unique unique (tenant_id, scope_key)
);

create index if not exists policies_tenant_idx on dojo.policies(tenant_id);

comment on table dojo.policies is
'Per-scope governance policy set by an admin (the policy grid): attribution
default, confidentiality rules, and retention window for a scope. Policy edits
take effect on the next batch. Engagements may layer overrides on top (see
dojo.engagements.policy_overrides).';

comment on column dojo.policies.scope_key
     is 'The scope this policy applies to (e.g. all-org, team:mobile, stack:rust).';
comment on column dojo.policies.attribution_default
     is 'Default credit at this scope: named | anonymous. Source-dereference is a separate always-on invariant.';
comment on column dojo.policies.confidentiality
     is 'Confidentiality rules for the scope (dereference requirements, compliance pack e.g. HIPAA/PCI/SOC2).';
comment on column dojo.policies.retention_days
     is 'Retention window in days for artifacts/audit at this scope (null = tenant default).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.policies enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists policies_service_only on dojo.policies;
create policy policies_service_only on dojo.policies
    for all to authenticated, anon
    using (false) with check (false);
