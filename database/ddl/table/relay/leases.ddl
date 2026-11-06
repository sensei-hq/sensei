set search_path to relay, extensions;

-- Liveness and build slots in one table. A lane lease says its lane is alive;
-- a resource lease says its lane holds a machine-wide slot. History is kept:
-- only the granted row per resource is unique.
create table if not exists relay.leases (
  user_id     uuid              not null,
  id          uuid              not null,
  crew_id     uuid              not null,
  lane_id     uuid              not null,
  kind        relay.lease_kind  not null,
  resource    text,
  state       relay.lease_state not null,
  expires_at  timestamptz,
  granted_at  timestamptz,
  released_at timestamptz,
  props       jsonb             not null default '{}',
  constraint leases_pkey primary key (user_id, id),
  constraint leases_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint leases_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint leases_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id),
  constraint leases_kind_resource check ((kind = 'lane') = (resource is null))
);

create unique index if not exists leases_one_grant_per_resource
  on relay.leases (resource) where kind = 'resource' and state = 'granted';

alter table relay.leases enable row level security;

comment on table relay.leases is
'Lane liveness + resource slots. The granted-per-resource unique is machine-wide (no user_id).';
comment on column relay.leases.resource is 'The slot (rust-build, e2e). Null for a lane lease, always.';
comment on column relay.leases.granted_at is 'Null until granted: a queued lease claims no grant.';
comment on column relay.leases.props is 'Grant facts: disk/swap at grant, the refusal reason. Empty until set.';
