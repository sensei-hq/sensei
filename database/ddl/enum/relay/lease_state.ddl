set search_path to relay, extensions;

-- Where a lease stands: waiting, held, given back, lapsed, or turned down.
create type relay.lease_state as enum (
  'queued',
  'granted',
  'released',
  'expired',
  'refused'
);

comment on type relay.lease_state is
'At most one granted lease per resource, machine-wide; history rows keep their state.';
