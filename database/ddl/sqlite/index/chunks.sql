-- chunks
-- Chunked content with vector embeddings for semantic search.
-- Replaces the separate embeddings table from the Postgres schema.
-- embedding stored as blob in sqlite-vec F32_BLOB(384) format.
-- When sqlite-vec is not loaded, embedding is null and semantic ranking
-- falls back to FTS5 keyword search only.
--
-- type: symbol (from symbols table) | doc (from doc_sections) | lib (from library docs)

create table if not exists chunks (
  id          text    not null primary key
, file_path   text    not null
, chunk_index integer not null default 0
, type        text    not null default 'symbol'
                      check (type in ('symbol','doc','lib'))
, content     text    not null     -- text used to generate the embedding
, embedding   blob                 -- sqlite-vec F32_BLOB(384); null if unavailable
, token_count integer
, metadata    text                 -- JSON: {symbolId, heading, contentHash}
, modified_at text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text    not null default 'system'
, unique(file_path, chunk_index, type)
);

create index if not exists chunks_file_path_idx on chunks(file_path);
create index if not exists chunks_type_idx      on chunks(type);

-- sqlite-vec virtual table for vector similarity search.
-- Created at runtime by the application if sqlite-vec is loaded.
-- DDL shown here for documentation; not executed by dbd.
--
-- create virtual table if not exists chunks_vec using vec0(
--   id text primary key,
--   embedding float[384]
-- );
