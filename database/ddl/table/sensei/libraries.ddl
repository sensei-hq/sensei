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
