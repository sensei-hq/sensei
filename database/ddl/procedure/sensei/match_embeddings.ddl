set search_path to sensei, extensions;

-- Vector similarity search against file embeddings.
-- Requires the 'vector' extension and populated embeddings table.
create or replace function match_embeddings(
  p_repo_id        uuid,
  query_embedding  vector(384),
  match_threshold  float   default 0.3,
  match_count      integer default 20
)
returns table(file_path text, similarity float)
language sql
stable
as $$
  select
    e.file_path,
    (1 - (e.embedding <=> query_embedding))::float as similarity
  from sensei.embeddings e
  where e.repo_id = p_repo_id
    and (1 - (e.embedding <=> query_embedding)) >= match_threshold
  order by e.embedding <=> query_embedding
  limit match_count;
$$;
