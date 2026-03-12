set search_path to sensei, extensions;

create table if not exists chunks (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, file_path   text not null
, chunk_index integer not null
, content     text not null
, embedding   vector(384)
, token_count integer
, metadata    jsonb
, modified_at timestamptz not null default now()
, unique(repo_id, file_path, chunk_index)
);

create index if not exists chunks_repo_id_idx on chunks(repo_id);

create index if not exists idx_chunks_embedding_hnsw
  on chunks using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64);

comment on table chunks is
'Chunked content with embeddings. Replaces .sensei/chunks.json + .sensei/embeddings.json.
- HNSW index for fast vector similarity search (cosine distance)
- 384-dim embeddings from local model
- metadata: {type: "symbol"|"doc", contentHash, tf}';
