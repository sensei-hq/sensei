set search_path to sensei, extensions;

grant select, insert, update, delete on context_packs to authenticated, service_role;

alter table context_packs enable row level security;
drop policy if exists context_packs_select on context_packs;
drop policy if exists context_packs_write  on context_packs;
create policy context_packs_select on context_packs
  for select to authenticated using (public.can_access_repo(repo_id));
create policy context_packs_write on context_packs
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
