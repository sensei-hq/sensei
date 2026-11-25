set search_path to relay_archive, extensions;

-- Leases of closed crews (#393): same columns as relay.leases plus
-- archived_at; same primary key and row-local check; no foreign keys.
create table if not exists relay_archive.leases (
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
  archived_at timestamptz       not null default current_timestamp,
  constraint leases_pkey primary key (user_id, id),
  constraint leases_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint leases_kind_resource check ((kind = 'lane') = (resource is null))
);

create index if not exists leases_archived_at_idx
  on relay_archive.leases (archived_at);

alter table relay_archive.leases enable row level security;

comment on table relay_archive.leases is
'Leases of closed crews. Moved here at crew close; purged past 30 days.';
comment on column relay_archive.leases.archived_at is 'When the row moved here; the purge horizon counts from it.';
