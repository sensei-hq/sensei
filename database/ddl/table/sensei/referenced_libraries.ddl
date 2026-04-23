set search_path to sensei, extensions;

create table if not exists referenced_libraries (
  folder_id                uuid        not null references sensei.folders(id) on delete cascade
, library_id               uuid        not null references sensei.libraries(id) on delete cascade
, version_used             text
, props                    jsonb       not null default '{}'
, modified_at              timestamptz not null default now()
, created_at               timestamptz not null default now()
, primary key (folder_id, library_id)
);

create index if not exists referenced_libraries_library_id_idx
    on referenced_libraries(library_id);

comment on table referenced_libraries is
'Junction table linking folders (git/subtree folders) to their library dependencies.
- version_used: the pinned version observed in the folder (from package.json, Cargo.toml, etc.)
- props: extensible — {source:"Cargo.toml", usage_count, skill_path, skill_generated_at, ...}';

comment on column referenced_libraries.folder_id
     is 'Foreign key to folders — the folder that uses this library.';
comment on column referenced_libraries.library_id
     is 'Foreign key to libraries — the library dependency.';
comment on column referenced_libraries.version_used
     is 'Version of the library pinned in this folder, as observed from the package manifest.';
comment on column referenced_libraries.props
     is 'Extensible metadata: {source, usage_count, skill_path, skill_generated_at, ...}.';
comment on column referenced_libraries.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column referenced_libraries.created_at
     is 'Timestamp when this library was first detected in this folder.';
