-- supabase/migrations/20260316000003_lib_phase_split.sql

-- 1. Drop objects that depend on the 768-dim embedding column
drop index if exists sensei.shared_lib_sections_embedding_idx;
-- Full signature required: PostgreSQL needs arg types to resolve the function
drop function if exists sensei.match_shared_lib_sections(uuid, text, vector(768), int);

-- 2. Change embedding dimension: 768 → 384
alter table sensei.shared_lib_sections drop column if exists embedding;
alter table sensei.shared_lib_sections add column embedding vector(384);

-- 3. Recreate IVFFlat index (384-dim, cosine)
create index if not exists shared_lib_sections_embedding_idx
  on sensei.shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

-- 4. Recreate RPC with 384-dim query vector
create or replace function sensei.match_shared_lib_sections(
  p_shared_lib_id uuid,
  p_component     text,
  query_embedding vector(384),
  match_count     int default 10
)
returns table (
  id          uuid,
  title       text,
  url         text,
  local_path  text,
  description text,
  content     text,
  source_type text,
  component   text,
  similarity  float
)
language sql stable
as $$
  select
    id, title, url, local_path, description, content, source_type, component,
    1 - (embedding <=> query_embedding) as similarity
  from sensei.shared_lib_sections
  where shared_lib_id = p_shared_lib_id
    and (p_component is null or component = p_component)
    and embedding is not null
  order by embedding <=> query_embedding
  limit match_count;
$$;

-- 5. Add 'github' to source_type check on shared_libs
alter table sensei.shared_libs
  drop constraint if exists shared_libs_source_type_check;
alter table sensei.shared_libs
  add constraint shared_libs_source_type_check
  check (source_type in ('llms.txt', 'http', 'local', 'github'));

-- 6. Add 'github' to source_type check on shared_lib_sections
alter table sensei.shared_lib_sections
  drop constraint if exists shared_lib_sections_source_type_check;
alter table sensei.shared_lib_sections
  add constraint shared_lib_sections_source_type_check
  check (source_type in ('llms.txt', 'http', 'local', 'github'));

-- 7. Add embed_status to shared_libs (null = never embedded)
alter table sensei.shared_libs
  add column if not exists embed_status text
  check (embed_status in ('pending', 'embedding', 'ready'));
