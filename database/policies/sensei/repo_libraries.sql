set search_path to sensei, extensions;

grant select on repo_libraries to authenticated, service_role;

alter table repo_libraries enable row level security;
drop policy if exists repo_libraries_select on repo_libraries;
create policy repo_libraries_select on repo_libraries
  for select to authenticated using (public.can_access_repo(repo_id));
