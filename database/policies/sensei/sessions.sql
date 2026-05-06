set search_path to sensei, extensions;

grant select, insert, update, delete on sessions to authenticated, service_role;

alter table sessions enable row level security;
drop policy if exists sessions_select on sessions;
drop policy if exists sessions_write  on sessions;
create policy sessions_select on sessions
  for select to authenticated using (public.can_access_repo(repo_id));
create policy sessions_write on sessions
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
