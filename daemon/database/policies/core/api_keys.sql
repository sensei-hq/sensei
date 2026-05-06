set search_path to core, extensions;

grant select, insert, update on api_keys to authenticated, service_role;

alter table api_keys enable row level security;
drop policy if exists api_keys_select on api_keys;
drop policy if exists api_keys_write  on api_keys;
create policy api_keys_select on api_keys
  for select to authenticated using (public.has_account_role(account_id, 'admin'));
create policy api_keys_write on api_keys
  for all to authenticated
  using (public.has_account_role(account_id, 'owner'))
  with check (public.has_account_role(account_id, 'owner'));
