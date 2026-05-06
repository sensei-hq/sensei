set search_path to core, extensions;

grant select, insert, update on team_members to authenticated, service_role;

alter table team_members enable row level security;
drop policy if exists tm_select on team_members;
drop policy if exists tm_write  on team_members;
create policy tm_select on team_members
  for select to authenticated
  using (
    exists (
      select 1 from teams t
      where t.id = team_id and public.is_account_member(t.account_id)
    )
  );
create policy tm_write on team_members
  for all to authenticated
  using (
    role = 'maintainer'
    or public.has_account_role(
      (select account_id from teams where id = team_id),
      'admin'
    )
  );
