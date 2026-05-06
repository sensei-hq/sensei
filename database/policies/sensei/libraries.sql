set search_path to sensei, extensions;

grant select on libraries to authenticated, service_role;

alter table libraries enable row level security;
drop policy if exists libraries_select on libraries;
create policy libraries_select on libraries
  for select to authenticated using (true);
