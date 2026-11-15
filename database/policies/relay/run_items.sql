-- RLS for relay.run_items: a row is readable by its owner, and by others only
-- by an explicit share (none yet). Writes are the daemon owner's.
alter table relay.run_items enable row level security;

drop policy if exists run_items_owner_read on relay.run_items;
create policy run_items_owner_read on relay.run_items
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.run_items to authenticated;
