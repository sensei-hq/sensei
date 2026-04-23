set search_path to sensei, extensions;

create table if not exists embeddings (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, chunk_text               text        not null
, embedding                vector(384)
, modified_at              timestamptz not null default now()
, unique(folder_id, file_path)
);

create index if not exists embeddings_folder_id_idx
    on embeddings(folder_id);

comment on table embeddings is
'Per-file vector embeddings for semantic context_pack ranking.
- chunk_text: concatenated symbol names and signatures used to build the embedding
- embedding: 384-dim vector from local model (all-MiniLM-L6-v2)';

comment on column embeddings.id
     is 'Surrogate primary key (UUID).';
comment on column embeddings.folder_id
     is 'Foreign key to folders — which repo this embedding belongs to.';
comment on column embeddings.file_path
     is 'Repo-relative path of the source file this embedding represents.';
comment on column embeddings.chunk_text
     is 'Concatenated symbol names and signatures used as input to generate the embedding.';
comment on column embeddings.embedding
     is '384-dimensional vector embedding for semantic ranking.';
comment on column embeddings.modified_at
     is 'Timestamp of the last modification to this row.';
