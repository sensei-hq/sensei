set search_path to sensei, extensions;

grant select, insert, update, delete on snapshots to authenticated, service_role;

alter table snapshots enable row level security;
drop policy if exists snapshots_select on snapshots;
drop policy if exists snapshots_write  on snapshots;
create policy snapshots_select on snapshots
  for select to authenticated using (public.can_access_repo(repo_id));
create policy snapshots_write on snapshots
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
