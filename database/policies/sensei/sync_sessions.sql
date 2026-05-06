set search_path to sensei, extensions;

grant select on sync_sessions to authenticated;
grant all    on sync_sessions to service_role;

alter table sync_sessions enable row level security;
drop policy if exists sync_sessions_select   on sync_sessions;
drop policy if exists sync_sessions_platform on sync_sessions;
create policy sync_sessions_select on sync_sessions
  for select to authenticated using (public.is_account_member(account_id));
create policy sync_sessions_platform on sync_sessions
  for select to authenticated using (public.auth_app_role() = 'platform_admin');
