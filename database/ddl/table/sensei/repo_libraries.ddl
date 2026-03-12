set search_path to sensei, extensions;

create table if not exists repo_libraries (
  repo_id      uuid not null references sensei.repos(id) on delete cascade
, library_id   uuid not null references sensei.libraries(id) on delete cascade
, version_used text
, primary key (repo_id, library_id)
);

create index if not exists repo_libraries_library_id_idx on repo_libraries(library_id);

comment on table repo_libraries is
'Junction table linking repos to their known library dependencies.
- version_used: the pinned version observed in the repo (e.g. from package.json)';
