set search_path to core, extensions;

grant select on profile_accounts to authenticated, service_role;

alter table profile_accounts enable row level security;
drop policy if exists pa_select_own   on profile_accounts;
drop policy if exists pa_select_admin on profile_accounts;
create policy pa_select_own on profile_accounts
  for select to authenticated using (user_id = auth.uid());
create policy pa_select_admin on profile_accounts
  for select to authenticated using (public.has_account_role(account_id, 'admin'));
