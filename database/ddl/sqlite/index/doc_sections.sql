-- doc_sections
-- Markdown/MDX documentation sections from the repo.
-- Used by SectionSlicer for context_pack assembly.
-- code_refs: JSON array of file paths mentioned in the section content.

create table if not exists doc_sections (
  id          text    not null primary key
, file_path   text    not null
, heading     text                       -- null for preamble before first heading
, level       integer not null default 1 -- 1=H1, 2=H2, etc.
, start_line  integer not null
, end_line    integer
, content     text    not null
, code_refs   text    not null default '[]'  -- JSON: ["src/auth.ts"]
, modified_at text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text    not null default 'system'
, unique(file_path, start_line)
);

create index if not exists doc_sections_file_path_idx on doc_sections(file_path);

create virtual table if not exists doc_sections_fts using fts5(
  id unindexed
, heading
, content
, content='doc_sections'
, content_rowid='rowid'
);
