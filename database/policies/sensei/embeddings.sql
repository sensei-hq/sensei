set search_path to sensei, extensions;

grant select on embeddings to authenticated;
grant all    on embeddings to service_role;

alter table embeddings enable row level security;
drop policy if exists embeddings_select on embeddings;
create policy embeddings_select on embeddings
  for select to authenticated using (public.can_access_repo(repo_id));
