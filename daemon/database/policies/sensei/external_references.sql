set search_path to sensei, extensions;

grant select on external_references to authenticated, service_role;

alter table external_references enable row level security;
drop policy if exists references_select on external_references;
create policy references_select on external_references
  for select to authenticated using (true);
