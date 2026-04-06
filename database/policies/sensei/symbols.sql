set search_path to sensei, extensions;

grant select on symbols to authenticated;
grant all    on symbols to service_role;

alter table symbols enable row level security;
drop policy if exists symbols_select on symbols;
create policy symbols_select on symbols
  for select to authenticated using (public.can_access_repo(repo_id));
