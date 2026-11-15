-- RLS for relay.lanes: a row is readable by its owner, and by others only by an
-- explicit share (slice 1 has no shares). Writes are the daemon owner's: the
-- dōjō's person-posted writes arrive with slice 2's policies.
alter table relay.lanes enable row level security;

drop policy if exists lanes_owner_read on relay.lanes;
create policy lanes_owner_read on relay.lanes
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.lanes to authenticated;
