set search_path to sensei, extensions;

create table if not exists chunks (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, chunk_index              integer     not null
, content                  text        not null
, embedding                vector(384)
, token_count              integer
, metadata                 jsonb
, modified_at              timestamptz not null default now()
, unique(folder_id, file_path, chunk_index)
);

create index if not exists chunks_folder_id_idx
    on chunks(folder_id);

create index if not exists idx_chunks_embedding_hnsw
    on chunks using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
 where embedding is not null;

comment on table chunks is
'Chunked content with embeddings for semantic search.
- HNSW index for fast vector similarity search (cosine distance)
- 384-dim embeddings from local model
- metadata: {type: "symbol"|"doc", contentHash, tf}';

comment on column chunks.id
     is 'Surrogate primary key (UUID).';
comment on column chunks.folder_id
     is 'Foreign key to folders — which repo this chunk belongs to.';
comment on column chunks.file_path
     is 'Repository-relative path of the source file this chunk was extracted from.';
comment on column chunks.chunk_index
     is 'Zero-based position of this chunk within its source file.';
comment on column chunks.content
     is 'Raw text content of this chunk as extracted from the source file.';
comment on column chunks.embedding
     is '384-dimensional vector embedding of the chunk content for semantic similarity search.';
comment on column chunks.token_count
     is 'Number of tokens in this chunk as counted by the embedding model tokenizer.';
comment on column chunks.metadata
     is 'JSON metadata for the chunk, e.g. {type: "symbol"|"doc", contentHash, tf}.';
comment on column chunks.modified_at
     is 'Timestamp of the last modification to this row.';
