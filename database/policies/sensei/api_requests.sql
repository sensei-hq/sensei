set search_path to sensei, extensions;

grant select on api_requests to authenticated;
grant all    on api_requests to service_role;

alter table api_requests enable row level security;
drop policy if exists api_requests_select   on api_requests;
drop policy if exists api_requests_platform on api_requests;
create policy api_requests_select on api_requests
  for select to authenticated using (public.can_access_repo(repo_id));
create policy api_requests_platform on api_requests
  for select to authenticated using (public.auth_app_role() = 'platform_admin');
