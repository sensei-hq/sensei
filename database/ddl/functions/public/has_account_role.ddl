set search_path to public, extensions;

-- Returns true if the current user holds at least the specified role in an account.
-- Role hierarchy: owner > admin > member.
create or replace function has_account_role(p_account_id uuid, p_role text)
returns boolean language sql stable security definer as
$$
  select exists (
    select 1 from core.profile_accounts
    where user_id    = auth.uid()
      and account_id = p_account_id
      and case p_role
            when 'member' then role in ('member', 'admin', 'owner')
            when 'admin'  then role in ('admin',  'owner')
            when 'owner'  then role =  'owner'
            else false
          end
  )
$$;

grant execute on function has_account_role(uuid, text) to authenticated, service_role;
