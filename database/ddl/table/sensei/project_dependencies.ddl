set search_path to sensei, extensions;

create table if not exists project_dependencies (
  from_project_id  uuid          not null references sensei.projects(id) on delete cascade
, to_project_id    uuid          not null references sensei.projects(id) on delete cascade
, from_folder_id   uuid          not null references sensei.folders(id)  on delete cascade
, source_protocol  text          not null
, source_manifest  text          not null
, resolved_target  text
, modified_at      timestamptz   not null default now()
, primary key (from_project_id, to_project_id, from_folder_id, source_manifest)
, check (from_project_id <> to_project_id)
);

create index if not exists project_dependencies_from_idx
    on project_dependencies(from_project_id);

create index if not exists project_dependencies_to_idx
    on project_dependencies(to_project_id);

create index if not exists project_dependencies_from_folder_idx
    on project_dependencies(from_folder_id);

comment on table project_dependencies is
'First-class project→project edges detected via local-path protocols.
When a manifest declares a dep with `link:` / `workspace:` / `file:` (npm) or
`path=` (Cargo), the referenced target folder is looked up in sensei.folders.
If it belongs to a DIFFERENT sensei.projects row than the declaring folder,
we record the edge here (from → to) instead of writing an external-library
row to referenced_libraries. Powers the project window "depends on other
project" surface (Track 3 Libraries screen) and lets the daemon reason about
inner-source dependency graphs.
- from_project_id: the project that DECLARES the dep
- to_project_id:   the project the dep RESOLVES to
- from_folder_id:  which folder inside from_project holds the manifest
- source_protocol: link | workspace | file | path
- source_manifest: package.json | Cargo.toml | pyproject.toml | ...
- resolved_target: the raw local-source string (../actions, workspace:*, etc.)';

comment on column project_dependencies.from_project_id
     is 'FK to projects — the project whose manifest names the dep.';
comment on column project_dependencies.to_project_id
     is 'FK to projects — the project the local-protocol dep resolves to.';
comment on column project_dependencies.from_folder_id
     is 'FK to folders — the folder inside from_project that carries the manifest.';
comment on column project_dependencies.source_protocol
     is 'Local-dep protocol: link (npm link:), workspace (npm workspace:), file (npm file:), path (Cargo path=).';
comment on column project_dependencies.source_manifest
     is 'Which manifest file declared the edge (package.json, Cargo.toml, pyproject.toml).';
comment on column project_dependencies.resolved_target
     is 'Raw local-source payload as written in the manifest (e.g. "../actions", "workspace:*").';
comment on column project_dependencies.modified_at
     is 'Timestamp of the last modification to this row.';
