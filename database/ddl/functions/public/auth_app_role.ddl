set search_path to public, extensions;

-- Returns the app-level role from the JWT app_metadata claim (e.g. 'platform_admin').
create or replace function auth_app_role()
returns text language sql stable as
$$ select coalesce((auth.jwt() -> 'app_metadata' ->> 'role'), 'member') $$;

grant execute on function auth_app_role() to authenticated, service_role;
