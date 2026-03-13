-- supabase/migrations/20260313000001_phase2_context_pack.sql

-- pgvector extension for semantic search
create extension if not exists vector;

-- Embeddings: one row per file, embedding via nomic-embed-text (768-dim)
-- Embedding model: nomic-embed-text (pull with: ollama pull nomic-embed-text)
create table if not exists sensei.embeddings (
  repo_id    uuid not null references sensei.repos(id) on delete cascade,
  file_path  text not null,
  chunk_text text not null,
  embedding  vector(768) not null,
  updated_at timestamptz not null default now(),
  primary key (repo_id, file_path)
);
create index if not exists idx_embeddings_vector
  on sensei.embeddings using ivfflat (embedding vector_cosine_ops)
  with (lists = 100);

-- BM25 full-text search: add generated tsvector column to symbols
alter table sensei.symbols
  add column if not exists search_vec tsvector generated always as (
    to_tsvector('english',
      coalesce(name, '') || ' ' ||
      coalesce(signature, '') || ' ' ||
      coalesce(docstring, ''))
  ) stored;
create index if not exists idx_symbols_search
  on sensei.symbols using gin(search_vec);

-- Doc sections: populated by MarkdownAdapter during indexing
create table if not exists sensei.doc_sections (
  repo_id    uuid not null references sensei.repos(id) on delete cascade,
  file_path  text not null,
  heading    text not null,
  level      integer not null,
  start_line integer not null,
  end_line   integer not null,
  content    text not null,
  code_refs  text[] not null default '{}',
  primary key (repo_id, file_path, start_line)
);
create index if not exists idx_doc_sections_repo
  on sensei.doc_sections(repo_id, file_path);

-- Context packs: persisted results for dashboard inspector
create table if not exists sensei.context_packs (
  id           uuid primary key default gen_random_uuid(),
  repo_id      uuid not null references sensei.repos(id) on delete cascade,
  session_id   text,
  task         text not null,
  model_id     text,
  slices       jsonb not null default '[]',
  total_tokens integer not null default 0,
  created_at   timestamptz not null default now()
);
create index if not exists idx_context_packs_repo_session
  on sensei.context_packs(repo_id, session_id);

-- SQL helper: BFS rank from changed files through import graph
create or replace function sensei.rank_bfs(
  p_repo_id uuid,
  p_changed_files text[]
)
returns table(file_path text, score double precision)
language sql
security definer
as $$
  WITH RECURSIVE bfs AS (
    SELECT p_cf AS file, 2.0::double precision AS score, 0 AS depth
    FROM unnest(p_changed_files) AS p_cf
    UNION ALL
    SELECT i.target_path, (bfs.score * 0.65)::double precision, bfs.depth + 1
    FROM sensei.imports i
    JOIN bfs ON i.source_file = bfs.file
    WHERE i.repo_id = p_repo_id
      AND bfs.depth < 4
  )
  SELECT file AS file_path, MAX(score)::double precision AS score
  FROM bfs
  GROUP BY file
$$;

-- SQL helper: BM25 rank via tsvector full-text search
create or replace function sensei.rank_bm25(
  p_repo_id uuid,
  p_query text
)
returns table(file_path text, score double precision)
language sql
security definer
as $$
  SELECT file_path, MAX(ts_rank(search_vec, plainto_tsquery('english', p_query)))::double precision AS score
  FROM sensei.symbols
  WHERE repo_id = p_repo_id
    AND search_vec @@ plainto_tsquery('english', p_query)
  GROUP BY file_path
  ORDER BY score DESC
  LIMIT 20
$$;

-- SQL helper: semantic similarity via pgvector
create or replace function sensei.match_embeddings(
  p_repo_id uuid,
  query_embedding vector(768),
  match_threshold double precision,
  match_count int
)
returns table(file_path text, similarity double precision)
language sql
security definer
as $$
  SELECT file_path, (1 - (embedding <=> query_embedding))::double precision AS similarity
  FROM sensei.embeddings
  WHERE repo_id = p_repo_id
    AND 1 - (embedding <=> query_embedding) > match_threshold
  ORDER BY embedding <=> query_embedding
  LIMIT match_count
$$;

-- Grants (same pattern as Phase 1)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
