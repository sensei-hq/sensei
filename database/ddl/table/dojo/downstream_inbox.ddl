set search_path to dojo, extensions;

create table if not exists dojo.downstream_inbox (
  id            uuid             primary key default gen_random_uuid()
, tenant_id     uuid             not null references dojo.tenants(id)
, membership_id uuid             not null references dojo.memberships(id)
, artifact_id   uuid             not null references dojo.artifacts(id)
, scope         jsonb            not null default '{}'
, attribution   jsonb            not null default '{}'
, state         dojo.inbox_state not null default 'pending'
, delivered_at  timestamptz
, acted_at      timestamptz
, created_at    timestamptz      not null default now()
, updated_at    timestamptz      not null default now()
, constraint downstream_inbox_member_artifact_unique unique (membership_id, artifact_id)
);

create index if not exists downstream_inbox_tenant_idx on dojo.downstream_inbox(tenant_id);
create index if not exists downstream_inbox_membership_idx on dojo.downstream_inbox(membership_id);
create index if not exists downstream_inbox_artifact_idx on dojo.downstream_inbox(artifact_id);

comment on table dojo.downstream_inbox is
'The distribution ledger: one row per (membership, published artifact) fanned
out to every membership whose scope matches the artifact. Tracks delivery
(delivered_at) and the consumer''s fed-back local decision (state:
pending/applied/muted/pinned), which rolls up into the artifact''s landing
telemetry. The daemon mirrors this into a local sensei-side inbox that the
Upgrades screen reads; mute/pin overrides are honoured locally (C7).';

comment on column dojo.downstream_inbox.state
     is 'Consumer decision fed back for telemetry: pending | applied | muted | pinned.';
comment on column dojo.downstream_inbox.delivered_at
     is 'When the artifact was delivered to the membership''s daemon.';
comment on column dojo.downstream_inbox.acted_at
     is 'When the consumer applied/muted/pinned it.';
