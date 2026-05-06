set search_path to sensei, extensions;

grant select on imports to authenticated;
grant all    on imports to service_role;

alter table imports enable row level security;
drop policy if exists imports_select on imports;
create policy imports_select on imports
  for select to authenticated using (public.can_access_repo(repo_id));
