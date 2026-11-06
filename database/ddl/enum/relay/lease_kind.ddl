set search_path to relay, extensions;

-- What a lease holds: a lane's liveness, or a machine-wide build slot.
create type relay.lease_kind as enum (
  'lane',
  'resource'
);

comment on type relay.lease_kind is
'A lane lease names no resource; a resource lease must. Leases never leave the machine.';
