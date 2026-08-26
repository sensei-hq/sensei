set search_path to staging, dojo, extensions;

-- Seed dojo.tenants from staging.tenants (dbd auto-runs this on import).
--
-- Replaces dojo.seed_global_dojo(), which held the same row as literal VALUES in
-- plpgsql. Every seeded row in this project now comes from a datafile, so there
-- is no second mechanism to remember — and no procedure that can silently drift
-- from the table it writes to, which is exactly how seed_ponytail_pack came to
-- reference a column that had been renamed.
--
-- Incremental and non-destructive: `on conflict (key)` updates only when the
-- datafile's timestamp is at least the live row's, so re-importing unchanged
-- data re-applies identical values and a row edited more recently in prod is
-- never clobbered. Same guard as import_rule_packs/import_scopes.
--
-- The id is SEEDED, not generated. dojo.tenants.id defaults to
-- gen_random_uuid(), so a row inserted by an import gets a fresh uuid on every
-- plane and on every reset-and-redeploy — a different id each time for the same
-- global tenant, orphaning everything that referenced the previous one. The
-- global dōjō is global by definition; its id has to be too.
--
-- `settings` is deliberately not seeded — it is tenant-owned state, and the
-- seed has no business resetting it on every deploy.
create or replace procedure import_tenants()
language plpgsql as $$
begin
  insert into dojo.tenants (id, key, origin, org, dojo, scope, name, dojo_url, self_hosted, updated_at)
  select
      stg.id::uuid
    , stg.key
    , stg.origin::dojo.tenant_origin
    , stg.org
    , nullif(stg.dojo, '')
    , coalesce(stg.scope, 'private')::dojo.tenant_scope
    , stg.name
    , stg.dojo_url
    , coalesce(stg.self_hosted, false)
    , coalesce(stg.modified_at, now())
  from staging.tenants stg
  where stg.key is not null
  on conflict (key)
  do update set
      origin      = excluded.origin
    , org         = excluded.org
    , dojo        = excluded.dojo
    , scope       = excluded.scope
    , name        = excluded.name
    , dojo_url    = excluded.dojo_url
    , self_hosted = excluded.self_hosted
    , updated_at  = excluded.updated_at
  where excluded.updated_at >= dojo.tenants.updated_at;
end;
$$;
