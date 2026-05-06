set search_path to sensei, extensions;

grant select on scan_state to authenticated;
grant all    on scan_state to service_role;

alter table scan_state enable row level security;
drop policy if exists scan_state_select on scan_state;
create policy scan_state_select on scan_state
  for select to authenticated using (public.can_access_repo(repo_id));
