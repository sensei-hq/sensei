set search_path to sensei, extensions;

create table if not exists doc_sections (
  id          uuid    primary key default gen_random_uuid()
, repo_id     uuid    not null references sensei.repos(id) on delete cascade
, file_path   text    not null
, heading     text
, level       integer not null default 1
, start_line  integer not null
, end_line    integer
, content     text    not null
, code_refs   text[]  not null default '{}'
, unique(repo_id, file_path, start_line)
);

create index if not exists doc_sections_repo_id_idx   on doc_sections(repo_id);
create index if not exists doc_sections_file_path_idx on doc_sections(repo_id, file_path);

comment on table doc_sections is
'Markdown/MDX document sections written by the engine pipeline.
- heading: section heading text (null for preamble before first heading)
- level: heading level (1=H1, 2=H2, etc.)
- code_refs: file paths mentioned in the section content
Used by SectionSlicer for context_pack assembly.';
