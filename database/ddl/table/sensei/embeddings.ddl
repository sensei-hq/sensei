set search_path to sensei, extensions;

create table if not exists embeddings (
  id          uuid        primary key default gen_random_uuid()
, repo_id     uuid        not null references sensei.repos(id) on delete cascade
, file_path   text        not null
, chunk_text  text        not null
, embedding   vector(384)
, updated_at  timestamptz not null default now()
, unique(repo_id, file_path)
);

create index if not exists embeddings_repo_id_idx on embeddings(repo_id);

comment on table embeddings is
'Per-file vector embeddings for semantic context_pack ranking.
- chunk_text: concatenated symbol names and signatures used to build the embedding
- embedding: 384-dim vector from the TransformersBackend (all-MiniLM-L6-v2)
Optional: only written when a backend is provided to indexRepo().';
