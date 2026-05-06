set search_path to sensei, extensions;

grant select, insert, update, delete on repo_libs to authenticated, service_role;

alter table repo_libs enable row level security;
drop policy if exists repo_libs_select on repo_libs;
drop policy if exists repo_libs_write  on repo_libs;
create policy repo_libs_select on repo_libs
  for select to authenticated using (public.can_access_repo(repo_id));
create policy repo_libs_write on repo_libs
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
