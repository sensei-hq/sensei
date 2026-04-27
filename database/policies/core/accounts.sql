set search_path to core, extensions;

grant select on accounts to authenticated, service_role;

alter table accounts enable row level security;
drop policy if exists accounts_select on accounts;
create policy accounts_select on accounts
  for select to authenticated using (public.is_account_member(id));
