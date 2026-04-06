set search_path to sensei, extensions;

create table if not exists embeddings (
  id          uuid        primary key default gen_random_uuid()
, repo_id     uuid        not null references sensei.repos(id) on delete cascade
, file_path   text        not null
, chunk_text  text        not null
, embedding   vector(384)
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
, unique(repo_id, file_path)
);

create index if not exists embeddings_repo_id_idx on embeddings(repo_id);

comment on table embeddings is
'Per-file vector embeddings for semantic context_pack ranking.
- chunk_text: concatenated symbol names and signatures used to build the embedding
- embedding: 384-dim vector from the TransformersBackend (all-MiniLM-L6-v2)
Optional: only written when a backend is provided to indexRepo().';

comment on column embeddings.id is 'Surrogate primary key (UUID).';
comment on column embeddings.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column embeddings.file_path is 'Repo-relative path of the source file this embedding represents.';
comment on column embeddings.chunk_text is 'Concatenated symbol names and signatures used as input to generate the embedding.';
comment on column embeddings.embedding is '384-dimensional vector embedding produced by all-MiniLM-L6-v2 for semantic ranking.';
comment on column embeddings.modified_at is 'Timestamp of the last modification to this row.';
comment on column embeddings.modified_by is 'Identity (user, role, or service) that last modified this row.';
