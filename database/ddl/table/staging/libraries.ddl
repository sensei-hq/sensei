set search_path to staging, extensions;

drop table if exists libraries cascade;
create table libraries (
  name              text
, ecosystem         text
, kind              text
, version           text
, description       text
, source_type       text
, base_url          text
, local_path        text
, homepage_url      text
, docs_url          text
, icons             jsonb       default '{}'
, props             jsonb       default '{}'
, tags              text[]      default '{}'
, modified_at       timestamptz not null default now()
);

create unique index if not exists libraries_ukey
    on libraries(ecosystem, name);

comment on table libraries is
'Staging buffer for sensei.libraries. Natural key: (ecosystem, name).';
