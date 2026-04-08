-- lib_doc_sections
-- Documentation chunks for indexed libraries.
-- embedding stored as blob (sqlite-vec F32_BLOB); null when sqlite-vec not loaded.
-- Shared across projects — a library's docs are indexed once, used by all projects.

create table if not exists lib_doc_sections (
  id           text    not null primary key
, library_id   text    not null references libraries(id) on delete cascade
, title        text    not null
, url          text
, local_path   text
, description  text    not null
, content      text
, source_type  text    not null check (source_type in ('llms.txt','http','local'))
, component    text
, embedding    blob                      -- sqlite-vec F32_BLOB(768); null if unavailable
, last_fetched text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, created_at   text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at  text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by  text    not null default 'system'
);

create index if not exists lib_doc_sections_library_idx  on lib_doc_sections(library_id);
create index if not exists lib_doc_sections_component_idx on lib_doc_sections(library_id, component)
  where component is not null;

-- Full-text search over library doc titles and descriptions
create virtual table if not exists lib_doc_sections_fts using fts5(
  id unindexed
, title
, description
, content='lib_doc_sections'
, content_rowid='rowid'
);
