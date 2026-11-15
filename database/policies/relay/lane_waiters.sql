-- RLS for relay.lane_waiters: a row is readable by its owner, and by others
-- only by an explicit share (none yet). Writes are the daemon owner's.
alter table relay.lane_waiters enable row level security;

drop policy if exists lane_waiters_owner_read on relay.lane_waiters;
create policy lane_waiters_owner_read on relay.lane_waiters
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.lane_waiters to authenticated;
