set search_path to sensei, extensions;

create table if not exists "references" (
  id          uuid primary key default gen_random_uuid()
, url         text not null unique
, title       text
, description text
, tags        text[]
, content     text          -- cached page content
, embedding   vector(384)   -- on title + description
, fetched_at  timestamptz
, modified_at timestamptz not null default now()
);

create index if not exists references_tags_idx on "references" using gin (tags);

create index if not exists idx_references_embedding_hnsw
  on "references" using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
  where embedding is not null;

comment on table "references" is
'External reference URLs with cached content and vector embedding.
- tags: free-form categorization array
- content: cached page body
- embedding: 384-dim vector on title + description for similarity search';
