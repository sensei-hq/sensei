set search_path to core, extensions;

grant select, insert, update on invitations to authenticated, service_role;

alter table invitations enable row level security;
drop policy if exists inv_select_own   on invitations;
drop policy if exists inv_select_admin on invitations;
drop policy if exists inv_write        on invitations;
create policy inv_select_own on invitations
  for select to authenticated
  using (email = (select email from auth.users where id = auth.uid()));
create policy inv_select_admin on invitations
  for select to authenticated using (public.has_account_role(account_id, 'admin'));
create policy inv_write on invitations
  for all to authenticated
  using (public.has_account_role(account_id, 'admin'))
  with check (public.has_account_role(account_id, 'admin'));
