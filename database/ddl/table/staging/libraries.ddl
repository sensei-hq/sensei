set search_path to staging;

create table if not exists libraries (
  id                  uuid
, name                text
, ecosystem           text
, version             text
, description         text
, homepage_url        text
, docs_url            text
, llms_txt_url        text
, llms_txt            text
, llms_txt_fetched_at timestamptz
, modified_at         timestamptz default now()
);

create unique index if not exists libraries_id_ukey on libraries(id);
