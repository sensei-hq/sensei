-- RLS for relay_archive.crews (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.crews enable row level security;

drop policy if exists crews_owner_read on relay_archive.crews;
create policy crews_owner_read on relay_archive.crews
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.crews to authenticated;
