-- libraries
-- Known third-party and internal libraries with optional documentation cache.
-- Shared across all projects (not project-scoped).
-- project_libraries join table links projects to the libraries they use.

create table if not exists libraries (
  id                  text    not null primary key
, name                text    not null
, ecosystem           text    not null check (ecosystem in ('npm','pypi','cargo','go','other'))
, version             text
, description         text
, homepage_url        text
, docs_url            text
, llms_txt_url        text
, llms_txt            text                          -- cached llms.txt content
, llms_txt_fetched_at text                          -- ISO 8601
, modified_at         text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by         text    not null default 'system'
, unique(ecosystem, name)
);

create index if not exists libraries_name_idx on libraries(name);

-- ─── project_libraries ───────────────────────────────────────────────────────
-- Join table: which libraries does each project use.
-- source_type mirrors the original custom_libs entry for how docs are fetched.

create table if not exists project_libraries (
  id          text not null primary key
, project_id  text not null references projects(id) on delete cascade
, library_id  text not null references libraries(id) on delete cascade
, source_type text not null check (source_type in ('llms.txt','http','local'))
, base_url    text
, unique(project_id, library_id)
);

create index if not exists project_libraries_project_idx  on project_libraries(project_id);
create index if not exists project_libraries_library_idx  on project_libraries(library_id);
