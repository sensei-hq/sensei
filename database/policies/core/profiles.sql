set search_path to core, extensions;

grant select, insert, update on profiles to authenticated, service_role;

alter table profiles enable row level security;
drop policy if exists profiles_select on profiles;
drop policy if exists profiles_update on profiles;
create policy profiles_select on profiles
  for select to authenticated using (true);
create policy profiles_update on profiles
  for update to authenticated using (user_id = auth.uid());
