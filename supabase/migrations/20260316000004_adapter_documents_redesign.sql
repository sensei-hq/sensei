-- supabase/migrations/20260316000004_adapter_documents_redesign.sql

-- ─── 1. Rename tables ─────────────────────────────────────────────────────────

alter table sensei.shared_libs         rename to libraries;
alter table sensei.shared_lib_sections rename to sections_in_document;
alter table sensei.repo_libs           rename to referenced_libraries;
alter table sensei.lib_queries         rename to queries_on_library;

-- ─── 2. Rename FK columns to match new names ─────────────────────────────────

-- referenced_libraries: shared_lib_id → library_id
alter table sensei.referenced_libraries rename column shared_lib_id to library_id;

-- queries_on_library: shared_lib_id → library_id
alter table sensei.queries_on_library rename column shared_lib_id to library_id;

-- sections_in_document: shared_lib_id → library_id (kept denormalized for query perf)
alter table sensei.sections_in_document rename column shared_lib_id to library_id;

-- ─── 3. Add document_count to libraries ──────────────────────────────────────

alter table sensei.libraries
  add column if not exists document_count int not null default 0;

-- ─── 4. Create documents_in_library ──────────────────────────────────────────

create table sensei.documents_in_library (
  id           uuid primary key default gen_random_uuid(),
  library_id   uuid not null references sensei.libraries(id) on delete cascade,
  sequence     int  not null default 0,
  title        text not null,
  url          text,
  local_path   text,
  summary      text not null default '',
  component    text,
  source_type  text not null,
  last_fetched timestamptz not null default now(),
  embedding    vector(384)
);

-- ─── 5. Add document_id + sequence to sections_in_document ───────────────────

alter table sensei.sections_in_document
  add column if not exists document_id uuid references sensei.documents_in_library(id) on delete cascade;

alter table sensei.sections_in_document
  add column if not exists sequence int not null default 0;

-- ─── 6. Remove columns moved to documents_in_library ─────────────────────────
-- (Do this AFTER creating documents_in_library, but sections without document_id
--  are orphans from old data — acceptable, they'll be re-indexed)

alter table sensei.sections_in_document drop column if exists url;
alter table sensei.sections_in_document drop column if exists local_path;
alter table sensei.sections_in_document drop column if exists source_type;
alter table sensei.sections_in_document drop column if exists component;
alter table sensei.sections_in_document drop column if exists description;

-- ─── 7. Drop old RPC ─────────────────────────────────────────────────────────

drop function if exists sensei.match_shared_lib_sections(uuid, text, vector(384), int);

-- ─── 8. Create new RPC match_libraries_sections ──────────────────────────────

create or replace function sensei.match_libraries_sections(
  p_library_id    uuid,
  p_component     text,
  query_embedding vector(384),
  match_count     int default 10
)
returns table (
  section_id    uuid,
  section_title text,
  content       text,
  similarity    float,
  doc_id        uuid,
  doc_title     text,
  url           text,
  local_path    text,
  component     text,
  summary       text
)
language sql stable
as $$
  select
    s.id          as section_id,
    s.title       as section_title,
    s.content,
    1 - (s.embedding <=> query_embedding) as similarity,
    d.id          as doc_id,
    d.title       as doc_title,
    d.url,
    d.local_path,
    d.component,
    d.summary
  from sensei.sections_in_document s
  join sensei.documents_in_library d on d.id = s.document_id
  where s.library_id = p_library_id
    and (p_component is null or d.component = p_component)
    and s.embedding is not null
  order by s.embedding <=> query_embedding
  limit match_count;
$$;

-- ─── 9. Index for document lookup ────────────────────────────────────────────

create index if not exists documents_in_library_library_id_idx
  on sensei.documents_in_library (library_id);

create index if not exists sections_in_document_document_id_idx
  on sensei.sections_in_document (document_id);
