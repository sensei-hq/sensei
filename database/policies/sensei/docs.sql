set search_path to sensei, extensions;

grant select on docs to authenticated;
grant all    on docs to service_role;

alter table docs enable row level security;
drop policy if exists docs_select on docs;
create policy docs_select on docs
  for select to authenticated using (public.can_access_repo(repo_id));
