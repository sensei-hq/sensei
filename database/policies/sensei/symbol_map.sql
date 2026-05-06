set search_path to sensei, extensions;

grant select on symbol_map to authenticated;
grant all    on symbol_map to service_role;

alter table symbol_map enable row level security;
drop policy if exists symbol_map_select on symbol_map;
create policy symbol_map_select on symbol_map
  for select to authenticated using (public.can_access_repo(repo_id));
