-- Migrate tenants off the GitHub-era origin labels onto the tenant-KIND ones,
-- and rewrite the discovery keys to match (spec dojo-auth-provisioning §IV.7).
--
-- `<origin>/<slug>` IS the key — dojo-auth.ts resolves a tenant by joining the
-- two URL segments — so changing `origin` without rewriting `key` would make
-- every affected tenant unreachable. Both move together, in one statement.
--
-- Idempotent: matches only rows still carrying the old labels, so re-running is
-- a no-op. Guarded on the type actually having the new values, so a partial
-- deploy fails quietly rather than erroring.
do $$
begin
  if not exists (
    select 1 from pg_enum e
      join pg_type t on t.oid = e.enumtypid
     where t.typname = 'tenant_origin' and e.enumlabel = 'organization'
  ) then
    raise notice 'tenant_origin has no `organization` label yet — skipping migration';
    return;
  end if;

  -- A tenant whose key already says `personal/` IS a personal dōjō, whatever
  -- the old origin claimed. `personal/jerry` carrying origin `org` is exactly
  -- the inconsistency D5 exists to fix.
  update dojo.tenants
     set origin = 'personal'::dojo.tenant_origin
   where origin::text in ('github', 'org')
     and key like 'personal/%';

  -- Everything else is an organization. The key's first segment becomes the new
  -- origin so `<origin>/<slug>` still resolves.
  update dojo.tenants
     set origin = 'organization'::dojo.tenant_origin
       , key    = 'organization/' || slug
   where origin::text in ('github', 'org')
     and key not like 'personal/%';
end $$;
