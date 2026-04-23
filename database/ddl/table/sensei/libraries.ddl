set search_path to sensei, extensions;

create type if not exists library_ecosystem
    as enum ('npm', 'pypi', 'cargo', 'go', 'docs');

create type if not exists library_source_type
    as enum ('llms.txt', 'http', 'local');

create type if not exists library_kind
    as enum ('detected', 'imported');

create table if not exists libraries (
  id                       uuid         primary key default gen_random_uuid()
, kind                     library_kind not null default 'detected'
, name                     text         not null
, ecosystem                library_ecosystem not null
, version                  text
, description              text
, source_type              library_source_type
, base_url                 text
, local_path               text
, homepage_url             text
, docs_url                 text
, page_count               integer      not null default 0
, embedding                vector(384)
, icons                    jsonb        not null default '{}'
, props                    jsonb        not null default '{}'
, tags                     text[]       not null default '{}'
, indexed_at               timestamptz
, modified_at              timestamptz  not null default now()
, unique(ecosystem, name)
);

create index if not exists libraries_kind_idx
    on libraries(kind);

create index if not exists libraries_embedding_hnsw
    on libraries using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
 where embedding is not null;

create index if not exists libraries_tags_idx
    on libraries using gin(tags);

comment on table libraries is
'Libraries — known packages and documentation sources.
Consolidates the former libraries, lib_meta, and shared_libs tables.
- kind: detected (from Cargo.toml, package.json) or imported (manual, internal SDKs, llms.txt)
- ecosystem: npm, pypi, cargo, go, or docs (for pure documentation sources)
- source_type: how pages are fetched — llms.txt, http, or local
- page_count: denormalized count of library_pages rows
- props: extensible — {llms_txt, llms_txt_fetched_at, skill_path, skill_generated_at, ...}
- embedding: 384-dim vector on description for similarity search';

comment on column libraries.id
     is 'Surrogate primary key (UUID).';
comment on column libraries.kind
     is 'Library classification: detected (from package manifests) or imported (manually registered).';
comment on column libraries.name
     is 'Package name within the ecosystem (e.g. "react", "tokio", "@lumen/icons").';
comment on column libraries.ecosystem
     is 'Package registry: npm, pypi, cargo, go, or docs (pure documentation source).';
comment on column libraries.version
     is 'Pinned or latest-known version string.';
comment on column libraries.description
     is 'Human-readable description of the library.';
comment on column libraries.source_type
     is 'How documentation pages are sourced: llms.txt, http, or local.';
comment on column libraries.base_url
     is 'Root URL for fetching documentation when source_type is http or llms.txt.';
comment on column libraries.local_path
     is 'Local filesystem path for reading documentation when source_type is local.';
comment on column libraries.homepage_url
     is 'URL of the library homepage or repository.';
comment on column libraries.docs_url
     is 'URL of the library primary documentation site.';
comment on column libraries.page_count
     is 'Denormalized count of library_pages rows for this library.';
comment on column libraries.embedding
     is '384-dimensional vector embedding on description for similarity search.';
comment on column libraries.icons
     is 'Display icons: {emoji, devicon, kanji, custom}.';
comment on column libraries.props
     is 'Extensible metadata: {llms_txt, llms_txt_fetched_at, skill_path, skill_generated_at, ...}.';
comment on column libraries.tags
     is 'Array of tag strings for filtering. Vocabulary controlled by sensei.tags table.';
comment on column libraries.indexed_at
     is 'Timestamp of the last successful index run for this library.';
comment on column libraries.modified_at
     is 'Timestamp of the last modification to this row.';
