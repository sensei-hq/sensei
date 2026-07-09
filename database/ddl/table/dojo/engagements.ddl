set search_path to dojo, extensions;

create table if not exists dojo.engagements (
  id               uuid                    primary key default gen_random_uuid()
, tenant_id        uuid                    not null references dojo.tenants(id)
, client           text                    not null
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

comment on table dojo.engagements is
'A registered client engagement, owned by a client-lead. Binds client work to
projects so it routes correctly and is dereferenced. Every artifact shared under
an engagement must carry dereferenced=true — the client-lead cannot per-item
override the universal strip; they audit that it held.';

comment on column dojo.engagements.client
     is 'Client / engagement name.';
comment on column dojo.engagements.project_bindings
     is 'Daemon-side project references routed to this engagement: [{project_id, name}]. Advisory (projects live in the daemon DB, Fork 1).';
comment on column dojo.engagements.policy_overrides
     is 'Engagement-specific overrides on the tenant policy (retention, confidentiality pack, etc.).';
comment on column dojo.engagements.status
     is 'active (in force, routes + dereferences) or ended (closed, retained for audit).';
