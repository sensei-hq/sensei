set search_path to staging, sensei, gateway, inference, activity, extensions;

create or replace procedure import_libraries()
language plpgsql
as $$
begin
  -- IDENTITY-LEVEL FACTS ONLY. S7b moved every version-scoped fact — version,
  -- source_type, base_url, docs_url, local_path, page_count — onto
  -- `library_versions`, and this procedure was not moved with it. It went on
  -- writing all six to `sensei.libraries`, so `dbd deploy` failed on any FRESH
  -- database with:
  --
  --   call staging.import_libraries() failed: column "version" of relation
  --   "libraries" does not exist
  --
  -- which took the Database health component to NeedsHumanAction, which left
  -- `daemon_start` skipped on `unmet_deps=[Database]` — so a first install came
  -- up with no daemon at all. Existing installs were provisioned before the
  -- drift and never re-ran the import, which is why it stayed invisible.
  insert into sensei.libraries (
      name, ecosystem, kind, description
    , homepage_url
    , icons, props, tags, modified_at
  )
  select
      stg.name
    , stg.ecosystem::sensei.library_ecosystem
    , coalesce(stg.kind, 'detected')::sensei.library_kind
    , stg.description
    , stg.homepage_url
    , coalesce(stg.icons, '{}')
    , coalesce(stg.props, '{}')
    , coalesce(stg.tags, '{}')
    , coalesce(stg.modified_at, now())
  from staging.libraries stg
  where stg.name is not null
    and stg.ecosystem is not null
  on conflict (ecosystem, name)
  do update set
      kind        = excluded.kind
    , description = excluded.description
    , homepage_url = excluded.homepage_url
    , icons       = excluded.icons
    , props       = excluded.props
    , tags        = excluded.tags
    , modified_at = excluded.modified_at
  where excluded.modified_at >= sensei.libraries.modified_at;

  -- THE VERSION-SCOPED HALF, to where S7b put it. Driven from here rather than
  -- from a second procedure because dbd does not order imports — the house
  -- convention after `import_rule_pack_rules` sorted before `import_rule_packs`
  -- and silently joined an empty table — so a dependent import runs inside the
  -- one that owns its parent.
  --
  -- Skipped for a row that states no version: a library IS identity and a
  -- version row with no version is not a fact, so there is nothing to write.
  insert into sensei.library_versions (
      library_id, version, source_type, base_url, docs_url, local_path
    , is_latest, modified_at
  )
  select
      lib.id
    , stg.version
    , stg.source_type::sensei.library_source_type
    , stg.base_url
    , stg.docs_url
    , stg.local_path
    -- The seed states one version per library, so it is that library's latest.
    -- `library_versions_one_latest` enforces at most one, and the upsert below
    -- re-asserts it rather than adding a second.
    , true
    , coalesce(stg.modified_at, now())
  from staging.libraries stg
  join sensei.libraries lib
    on lib.name = stg.name
   and lib.ecosystem = stg.ecosystem::sensei.library_ecosystem
  where stg.name is not null
    and stg.ecosystem is not null
    and stg.version is not null
  on conflict (library_id, version)
  do update set
      source_type = excluded.source_type
    , base_url    = excluded.base_url
    , docs_url    = excluded.docs_url
    , local_path  = excluded.local_path
    , modified_at = excluded.modified_at
  where excluded.modified_at >= sensei.library_versions.modified_at;
end;
$$;

comment on procedure import_libraries is
'Import staging.libraries into sensei.libraries (identity) and sensei.library_versions (version-scoped facts).
Upserts libraries on (ecosystem, name) and versions on (library_id, version); both update only if the source is newer (freshness gate).
Drives library_versions itself because dbd does not order imports.';
