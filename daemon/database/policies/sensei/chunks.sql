set search_path to sensei, extensions;

grant select on chunks to authenticated;
grant all    on chunks to service_role;

alter table chunks enable row level security;
drop policy if exists chunks_select on chunks;
create policy chunks_select on chunks
  for select to authenticated using (public.can_access_repo(repo_id));
