set search_path to sensei, extensions;

grant select on account_repos to authenticated, service_role;

-- security_barrier prevents WHERE-clause bypass attacks.
-- security_invoker=false lets the view cross user boundaries to show all
-- account members' repos; membership is enforced in the view's WHERE clause.
alter view account_repos set (security_barrier = true, security_invoker = false);
