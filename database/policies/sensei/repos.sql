set search_path to sensei, extensions;

grant select, insert, update, delete on repos to authenticated, service_role;

alter table repos enable row level security;
drop policy if exists repos_select on repos;
drop policy if exists repos_write  on repos;
create policy repos_select on repos
  for select to authenticated using (owner_id = auth.uid() or is_public = true);
create policy repos_write on repos
  for all to authenticated
  using (owner_id = auth.uid()) with check (owner_id = auth.uid());
