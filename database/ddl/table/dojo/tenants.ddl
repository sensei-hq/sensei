set search_path to dojo, extensions;

create table if not exists dojo.tenants (
  id            uuid              primary key default gen_random_uuid()
, key           text              not null
, origin        dojo.tenant_origin not null
, org           text              not null
, dojo          text
, scope         dojo.tenant_scope  not null default 'private'
, name          text              not null
, dojo_url      text              not null
, self_hosted   boolean           not null default false
, settings      jsonb             not null default '{}'
, created_at    timestamptz       not null default now()
, updated_at    timestamptz       not null default now()
, constraint tenants_key_unique unique (key)
);

comment on table dojo.tenants is
'A tenant Dōjō — the isolation boundary. Every other dojo.* row carries a
tenant_id back to here; cross-tenant reads are impossible by construction (the
service filters every query by tenant, Fork 1 in-query isolation). Addressed by
the discovery path <origin>/<org>/<dojo?>. The special global-dojo collective is
just a row here at scope=global (there is no separate "Collective" concept).';

comment on column dojo.tenants.key
     is 'Canonical discovery path, the unique key: "<origin>/<org>[/<dojo>]" (e.g. "github/sensei-hq", "org/global-dojo").';
comment on column dojo.tenants.origin
     is 'github (backed by a GitHub org identity) or org (custom-registered name).';
comment on column dojo.tenants.org
     is 'The GitHub org id (e.g. sensei-hq) or the custom org name.';
comment on column dojo.tenants.dojo
     is 'Optional sub-path when an org runs multiple Dōjōs (e.g. one per client engagement). Null for the org-level Dōjō.';
comment on column dojo.tenants.scope
     is 'private (normal org/client Dōjō) or global (the single public global-dojo everyone may join).';
comment on column dojo.tenants.dojo_url
     is 'Base URL the daemon connects to (SaaS dojo.sensei-hq.org/... or a self-hosted domain).';
comment on column dojo.tenants.self_hosted
     is 'True when the tenant runs on the customer''s own infra rather than SaaS.';
comment on column dojo.tenants.settings
     is 'Free-form tenant configuration (heartbeat window, triage thresholds, etc.).';
