-- RLS for relay_archive.runs (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.runs enable row level security;

drop policy if exists runs_owner_read on relay_archive.runs;
create policy runs_owner_read on relay_archive.runs
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.runs to authenticated;
