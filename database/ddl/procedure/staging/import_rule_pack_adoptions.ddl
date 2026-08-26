set search_path to staging, sensei, extensions;

-- Adopt the seeded library packs at their namespace (dbd auto-runs this on
-- import), replacing the tail of seed_default_constitution().
--
-- Runs AFTER import_rule_packs — dbd orders imports by the datafile list, and the
-- adoption references a pack by slug, so the pack must already be there. A
-- missing pack yields no adoption rather than an error: the join simply matches
-- nothing, which is the honest outcome for a datafile naming a pack that was not
-- shipped.
--
-- pinned_version is read from the pack's CURRENT version, exactly as the old
-- procedure did — an adoption pins what it adopted, and re-adopting is how a
-- namespace takes an update.
create or replace procedure import_rule_pack_adoptions()
language plpgsql as $$
begin
  -- The namespace first: an adoption cannot reference one that does not exist.
  -- Fixed id, seeded — see the note on staging.rule_pack_adoptions.
  insert into sensei.namespaces (id, scope_key, slug, name)
  select distinct stg.namespace_id::uuid, stg.namespace_scope, stg.namespace_slug, stg.namespace_name
    from staging.rule_pack_adoptions stg
   where stg.namespace_id is not null
  on conflict (scope_key, slug) do update set name = excluded.name;

  insert into sensei.rule_pack_adoptions (pack_id, namespace_id, pinned_version, adopted_by)
  select p.id, n.id, p.version, coalesce(stg.adopted_by, 'sensei')
    from staging.rule_pack_adoptions stg
    join sensei.rule_packs p
      on p.slug = stg.pack_slug and p.owner_namespace_id is null
    join sensei.namespaces n
      on n.scope_key = stg.namespace_scope and n.slug = stg.namespace_slug
  on conflict (pack_id, namespace_id) do nothing;
end;
$$;
