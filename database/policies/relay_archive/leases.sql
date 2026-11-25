-- RLS for relay_archive.leases (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.leases enable row level security;

drop policy if exists leases_owner_read on relay_archive.leases;
create policy leases_owner_read on relay_archive.leases
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.leases to authenticated;
