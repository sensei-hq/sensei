set search_path to sensei, extensions;

grant select, insert, update, delete on memory_items to authenticated, service_role;

alter table memory_items enable row level security;
drop policy if exists memory_items_select on memory_items;
drop policy if exists memory_items_write  on memory_items;
create policy memory_items_select on memory_items
  for select to authenticated using (public.can_access_repo(repo_id));
create policy memory_items_write on memory_items
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
