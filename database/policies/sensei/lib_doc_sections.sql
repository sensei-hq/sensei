set search_path to sensei, extensions;

grant select, insert, update, delete on lib_doc_sections to authenticated, service_role;

alter table lib_doc_sections enable row level security;
drop policy if exists lib_doc_sections_select on lib_doc_sections;
drop policy if exists lib_doc_sections_write  on lib_doc_sections;
create policy lib_doc_sections_select on lib_doc_sections
  for select to authenticated using (public.can_access_repo(repo_id));
create policy lib_doc_sections_write on lib_doc_sections
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
