set search_path to sensei, extensions;

grant select, insert, update, delete on task_sessions to authenticated, service_role;

alter table task_sessions enable row level security;
drop policy if exists task_sessions_select   on task_sessions;
drop policy if exists task_sessions_write    on task_sessions;
drop policy if exists task_sessions_platform on task_sessions;
create policy task_sessions_select on task_sessions
  for select to authenticated using (public.can_access_repo(repo_id));
create policy task_sessions_write on task_sessions
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
create policy task_sessions_platform on task_sessions
  for select to authenticated using (public.auth_app_role() = 'platform_admin');
