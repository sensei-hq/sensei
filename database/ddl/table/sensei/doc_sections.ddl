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
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
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

comment on column doc_sections.id is 'Surrogate primary key (UUID).';
comment on column doc_sections.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column doc_sections.file_path is 'Repository-relative path of the documentation file this section belongs to.';
comment on column doc_sections.heading is 'Text of the section heading; null for the preamble before the first heading.';
comment on column doc_sections.level is 'Heading depth level (1=H1, 2=H2, etc.).';
comment on column doc_sections.start_line is 'One-based line number where this section begins in the source file.';
comment on column doc_sections.end_line is 'One-based line number where this section ends; null if it extends to end of file.';
comment on column doc_sections.content is 'Full markdown text content of this section.';
comment on column doc_sections.code_refs is 'File paths referenced or mentioned within this section''s content.';
comment on column doc_sections.modified_at is 'Timestamp of the last modification to this row.';
comment on column doc_sections.modified_by is 'Identity (user, role, or service) that last modified this row.';
