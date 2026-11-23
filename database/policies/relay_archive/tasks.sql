-- RLS for relay_archive.tasks (#393): archived rows read like live ones — by
-- their owner. Writes are the daemon owner's (archive moves, purge deletes);
-- authenticated holds no write grant.
alter table relay_archive.tasks enable row level security;

drop policy if exists tasks_owner_read on relay_archive.tasks;
create policy tasks_owner_read on relay_archive.tasks
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay_archive.tasks to authenticated;
