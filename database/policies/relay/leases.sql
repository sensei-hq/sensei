-- RLS for relay.leases: a row is readable by its owner, and by others only by an
-- explicit share (slice 1 has no shares). Writes are the daemon owner's: the
-- dōjō's person-posted writes arrive with slice 2's policies.
alter table relay.leases enable row level security;

drop policy if exists leases_owner_read on relay.leases;
create policy leases_owner_read on relay.leases
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.leases to authenticated;
