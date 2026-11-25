-- RLS for relay_archive.run_items (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.run_items enable row level security;

drop policy if exists run_items_owner_read on relay_archive.run_items;
create policy run_items_owner_read on relay_archive.run_items
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.run_items to authenticated;
