set search_path to sensei, extensions;

grant select on shared_libs to authenticated, service_role;

alter table shared_libs enable row level security;
drop policy if exists shared_libs_select on shared_libs;
create policy shared_libs_select on shared_libs
  for select to authenticated using (true);
