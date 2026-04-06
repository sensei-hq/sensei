set search_path to staging;

create or replace procedure import_libraries()
language plpgsql
as $$
begin
  insert into sensei.libraries (
      id, name, ecosystem, version, description
    , homepage_url, docs_url, llms_txt_url, llms_txt
    , llms_txt_fetched_at, modified_at, modified_by
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.name, stg.ecosystem, stg.version, stg.description
    , stg.homepage_url, stg.docs_url, stg.llms_txt_url, stg.llms_txt
    , stg.llms_txt_fetched_at
    , coalesce(stg.modified_at, now())
    , coalesce(stg.modified_by, current_user)
  from staging.libraries stg
  where stg.name is not null
    and stg.ecosystem is not null
  on conflict (id)
  do update set
      name                = excluded.name
    , ecosystem           = excluded.ecosystem
    , version             = excluded.version
    , description         = excluded.description
    , homepage_url        = excluded.homepage_url
    , docs_url            = excluded.docs_url
    , llms_txt_url        = excluded.llms_txt_url
    , llms_txt            = excluded.llms_txt
    , llms_txt_fetched_at = excluded.llms_txt_fetched_at
    , modified_at         = excluded.modified_at
    , modified_by         = excluded.modified_by
  where excluded.modified_at >= sensei.libraries.modified_at;
end;
$$;
