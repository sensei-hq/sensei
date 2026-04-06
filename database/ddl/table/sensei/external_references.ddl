set search_path to sensei, extensions;

create table if not exists external_references (
  id          uuid primary key default gen_random_uuid()
, url         text not null unique
, title       text
, description text
, tags        text[]
, content     text          -- cached page content
, embedding   vector(384)   -- on title + description
, fetched_at  timestamptz
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
);

create index if not exists external_references_tags_idx on external_references using gin (tags);

create index if not exists external_references_embedding_hnsw
  on external_references using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
  where embedding is not null;

comment on table external_references is
'External reference URLs with cached content and vector embedding.
- tags: free-form categorization array
- content: cached page body
- embedding: 384-dim vector on title + description for similarity search';

comment on column external_references.id is 'Surrogate primary key (UUID).';
comment on column external_references.url is 'Unique URL of the external reference page.';
comment on column external_references.title is 'Title of the referenced page.';
comment on column external_references.description is 'Short description or excerpt summarizing the reference.';
comment on column external_references.tags is 'Free-form categorization tags for filtering and discovery.';
comment on column external_references.content is 'Cached full-page body content fetched from the URL.';
comment on column external_references.embedding is '384-dimensional vector embedding computed from title + description for similarity search.';
comment on column external_references.fetched_at is 'Timestamp when the page content was last successfully fetched.';
comment on column external_references.modified_at is 'Timestamp of the last modification to this row.';
comment on column external_references.modified_by is 'Identity (user, role, or service) that last modified this row.';
