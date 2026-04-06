set search_path to public, extensions;

-- Returns true if the current user can access a repo (owns it or it is public).
create or replace function can_access_repo(p_repo_id uuid)
returns boolean language sql stable security definer as
$$
  select exists (
    select 1 from sensei.repos
    where id = p_repo_id
      and (owner_id = auth.uid() or is_public = true)
  )
$$;

grant execute on function can_access_repo(uuid) to authenticated, service_role;
