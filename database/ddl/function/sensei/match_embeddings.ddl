set search_path to sensei, extensions;

drop function if exists match_embeddings cascade;

create or replace function match_embeddings(
  p_folder_id       uuid,
  query_embedding   vector(384),
  match_threshold   float   default 0.3,
  match_count      integer default 20
)
returns table(node_id uuid, file_path text, name text, kind node_kind, similarity float)
language sql
stable
as $$
  select
    n.id,
    n.file_path,
    n.name,
    n.kind,
    (1 - (n.embedding <=> query_embedding))::float as similarity
  from sensei.nodes n
  where n.folder_id = p_folder_id
    and n.embedding is not null
    and (1 - (n.embedding <=> query_embedding)) >= match_threshold
  order by n.embedding <=> query_embedding
  limit match_count;
$$;

comment on function match_embeddings is
'Vector similarity search against node embeddings.
Returns nodes with cosine similarity above the threshold.
Searches across all node kinds (files, functions, classes, sections, etc.).
p_folder_id: scope to a specific folder
query_embedding: 384-dim vector to search against
match_threshold: minimum similarity (0-1); default 0.3
match_count: max results to return; default 20';

grant execute on function match_embeddings(uuid, vector, float, integer) to authenticated, service_role;
