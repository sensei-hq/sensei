set search_path to core, extensions;

grant select, insert, update on teams to authenticated, service_role;

alter table teams enable row level security;
drop policy if exists teams_select on teams;
drop policy if exists teams_write  on teams;
create policy teams_select on teams
  for select to authenticated using (public.is_account_member(account_id));
create policy teams_write on teams
  for all to authenticated
  using (public.has_account_role(account_id, 'admin'))
  with check (public.has_account_role(account_id, 'admin'));
