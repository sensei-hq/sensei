set search_path to staging, sensei, gateway, inference, activity, extensions;

create or replace procedure import_libraries()
language plpgsql
as $$
begin
  insert into sensei.libraries (
      name, ecosystem, kind, version, description
    , source_type, base_url, local_path, homepage_url, docs_url
    , icons, props, tags, modified_at
  )
  select
      stg.name
    , stg.ecosystem::sensei.library_ecosystem
    , coalesce(stg.kind, 'detected')::sensei.library_kind
    , stg.version
    , stg.description
    , stg.source_type::sensei.library_source_type
    , stg.base_url
    , stg.local_path
    , stg.homepage_url
    , stg.docs_url
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
    , version     = excluded.version
    , description = excluded.description
    , source_type = excluded.source_type
    , base_url    = excluded.base_url
    , local_path  = excluded.local_path
    , homepage_url = excluded.homepage_url
    , docs_url    = excluded.docs_url
    , icons       = excluded.icons
    , props       = excluded.props
    , tags        = excluded.tags
    , modified_at = excluded.modified_at
  where excluded.modified_at >= sensei.libraries.modified_at;
end;
$$;

comment on procedure import_libraries is
'Import staging.libraries into sensei.libraries.
Upserts on (ecosystem, name), updates only if source is newer (freshness gate).';
