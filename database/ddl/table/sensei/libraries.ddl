set search_path to sensei, extensions;

create table if not exists libraries (
  id                  uuid primary key default gen_random_uuid()
, name                text not null
, ecosystem           text not null check (ecosystem in ('npm','pypi','cargo','go'))
, version             text
, description         text
, homepage_url        text
, docs_url            text
, llms_txt_url        text
, llms_txt            text          -- cached content
, llms_txt_fetched_at timestamptz
, embedding           vector(384)   -- on description
, modified_at         timestamptz not null default now()
, modified_by         text        not null default current_user
, unique(ecosystem, name)
);

create index if not exists idx_libraries_embedding_hnsw
  on libraries using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
  where embedding is not null;

comment on table libraries is
'Known libraries/packages with optional llms.txt and vector embedding.
- ecosystem: npm | pypi | cargo | go
- llms_txt: cached content from llms_txt_url
- embedding: 384-dim vector on description for similarity search';

comment on column libraries.id is 'Surrogate primary key (UUID).';
comment on column libraries.name is 'Package name within the ecosystem (e.g. "react", "lodash").';
comment on column libraries.ecosystem is 'Package registry the library belongs to: npm, pypi, cargo, or go.';
comment on column libraries.version is 'Pinned or latest-known version string for this library entry.';
comment on column libraries.description is 'Human-readable description of the library''s purpose.';
comment on column libraries.homepage_url is 'URL of the library''s official homepage or repository.';
comment on column libraries.docs_url is 'URL of the library''s primary documentation site.';
comment on column libraries.llms_txt_url is 'URL where the library''s llms.txt file can be fetched.';
comment on column libraries.llms_txt is 'Cached content fetched from llms_txt_url.';
comment on column libraries.llms_txt_fetched_at is 'Timestamp when llms_txt was last successfully fetched.';
comment on column libraries.embedding is '384-dimensional vector embedding computed from the description for similarity search.';
comment on column libraries.modified_at is 'Timestamp of the last modification to this row.';
comment on column libraries.modified_by is 'Identity (user, role, or service) that last modified this row.';
