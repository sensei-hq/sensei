set search_path to sensei, extensions;

grant select on doc_sections to authenticated;
grant all    on doc_sections to service_role;

alter table doc_sections enable row level security;
drop policy if exists doc_sections_select on doc_sections;
create policy doc_sections_select on doc_sections
  for select to authenticated using (public.can_access_repo(repo_id));
