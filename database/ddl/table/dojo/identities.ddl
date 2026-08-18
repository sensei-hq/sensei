set search_path to dojo, extensions;

create table if not exists dojo.identities (
  id            uuid             primary key default gen_random_uuid()
, tenant_id     uuid             not null references dojo.tenants(id)
, user_id       uuid             not null
, provider      dojo.auth_method not null
, subject       text             not null
, email         text
, display_name  text
, created_at    timestamptz      not null default now()
, last_login_at timestamptz
, constraint identities_provider_subject_unique unique (tenant_id, provider, subject)
);

create index if not exists identities_tenant_idx on dojo.identities(tenant_id);
create index if not exists identities_user_idx on dojo.identities(tenant_id, user_id);

comment on table dojo.identities is
'Projection of an external auth subject onto an internal user within a tenant.
Fork 1: the authoritative human identity lives in Supabase (auth only) — this
table records the mapping so the Dōjō service can resolve a verified subject to
a stable user_id and role. A user_id may have several identities (one per
provider); memberships reference user_id, not identities.id.';

comment on column dojo.identities.user_id
     is 'Stable internal user id (the Supabase auth subject). Referenced by dojo.memberships.user_id. Not a local FK — the user record is owned by Supabase.';
comment on column dojo.identities.provider
     is 'Which auth method produced this subject: sso, github_oauth, or device_code.';
comment on column dojo.identities.subject
     is 'The provider''s subject identifier (OIDC/SAML sub, GitHub user id, or device-code enrollment id).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.identities enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists identities_service_only on dojo.identities;
create policy identities_service_only on dojo.identities
    for all to authenticated, anon
    using (false) with check (false);
