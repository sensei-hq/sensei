set search_path to public, extensions;

-- Returns true if the current user is a member of the given account.
create or replace function is_account_member(p_account_id uuid)
returns boolean language sql stable security definer as
$$
  select exists (
    select 1 from core.profile_accounts
    where user_id = auth.uid() and account_id = p_account_id
  )
$$;

grant execute on function is_account_member(uuid) to authenticated, service_role;
