set search_path to public, extensions;

create or replace function auth_uid()
returns uuid language sql stable as
$$ select auth.uid() $$;

grant execute on function auth_uid() to authenticated, service_role;
