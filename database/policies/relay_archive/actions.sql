-- RLS for relay_archive.actions (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.actions enable row level security;

drop policy if exists actions_owner_read on relay_archive.actions;
create policy actions_owner_read on relay_archive.actions
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.actions to authenticated;
