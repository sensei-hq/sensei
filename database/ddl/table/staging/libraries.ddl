set search_path to staging, extensions;

create table if not exists libraries (
  id                uuid
, name              text
, ecosystem         text
, version           text
, description       text
, homepage_url      text
, docs_url          text
, llms_txt_url      text
, llms_txt          text
, llms_txt_fetched_at timestamptz
, created_at          timestamptz not null default now()
, modified_at         timestamptz not null default now()
, modified_by         text        not null default current_user
);

create unique index if not exists libraries_id_ukey
    on libraries(id);

comment on table libraries is
'Intermediate import buffer for bulk-loading rows into sensei.libraries.
Fields match sensei.libraries with staging-specific tracking columns.';

comment on column libraries.id is 'UUID that will become the surrogate primary key in sensei.libraries.';
comment on column libraries.name is 'Package name within the ecosystem (e.g. react, lodash).';
comment on column libraries.ecosystem is 'Package registry the library belongs to: npm, pypi, cargo, or go.';
comment on column libraries.version is 'Pinned or latest-known version string for this library entry.';
comment on column libraries.description is 'Human-readable description of the library''s purpose.';
comment on column libraries.homepage_url is 'URL of the library''s official homepage or repository.';
comment on column libraries.docs_url is 'URL of the library''s primary documentation site.';
comment on column libraries.llms_txt_url is 'URL where the library''s llms.txt file can be fetched.';
comment on column libraries.llms_txt is 'Cached content fetched from llms_txt_url.';
comment on column libraries.llms_txt_fetched_at is 'Timestamp when llms_txt was last successfully fetched.';
comment on column libraries.created_at is 'Timestamp when the staging row was inserted.';
comment on column libraries.modified_at is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column libraries.modified_by is 'Source-side modifier identity; passed through to sensei.libraries on upsert.';