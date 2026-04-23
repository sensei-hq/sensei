set search_path to sensei, extensions;

drop function if exists match_embeddings cascade;

create or replace function match_embeddings(
  p_repo_id         uuid,
  query_embedding   vector(384),
  match_threshold   float   default 0.3,
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

comment on function match_embeddings is
'Vector similarity search against file embeddings.
Requires the vector extension and populated embeddings table.
Returns files with cosine similarity above the threshold.
p_repo_id: scope to a specific repo
query_embedding: 384-dim vector to search against
match_threshold: minimum similarity (0-1); default 0.3
match_count: max results to return; default 20';

grant execute on function match_embeddings(uuid, vector, float, integer) to authenticated, service_role;