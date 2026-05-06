set search_path to platform, extensions;

grant select on ftr_distribution to authenticated, service_role;

-- security_barrier prevents policy bypass; security_invoker applies the
-- caller's JWT role so the platform_admin policies on base tables take effect.
alter view ftr_distribution set (security_barrier = true, security_invoker = true);
