set search_path to sensei, extensions;

create table if not exists repo_libraries (
  repo_id      uuid not null references sensei.repos(id) on delete cascade
, library_id   uuid not null references sensei.libraries(id) on delete cascade
, version_used text
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, primary key (repo_id, library_id)
);

create index if not exists repo_libraries_library_id_idx on repo_libraries(library_id);

comment on table repo_libraries is
'Junction table linking repos to their known library dependencies.
- version_used: the pinned version observed in the repo (e.g. from package.json)';

comment on column repo_libraries.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column repo_libraries.library_id is 'Foreign key to sensei.libraries — identifies the library dependency.';
comment on column repo_libraries.version_used is 'Version of the library pinned in this repo, as observed from the package manifest.';
comment on column repo_libraries.modified_at is 'Timestamp of the last modification to this row.';
comment on column repo_libraries.modified_by is 'Identity (user, role, or service) that last modified this row.';
