set search_path to sensei, extensions;

grant select on shared_lib_sections to authenticated, service_role;

alter table shared_lib_sections enable row level security;
drop policy if exists shared_lib_sections_select on shared_lib_sections;
create policy shared_lib_sections_select on shared_lib_sections
  for select to authenticated using (true);
