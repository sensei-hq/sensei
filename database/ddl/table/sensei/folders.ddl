set search_path to sensei, extensions;

create type if not exists folder_kind
    as enum ('parent', 'folder', 'git', 'subtree');

create table if not exists folders (
  id                       uuid        primary key default gen_random_uuid()
, root_id                  uuid        not null references sensei.folders_to_watch(id) on delete cascade
, parent_id                uuid        references sensei.folders(id) on delete cascade
, project_id               uuid        references sensei.projects(id) on delete set null
, kind                     folder_kind not null
, name                     text        not null
, path                     text        not null
, abs_path                 text        not null unique
, remote_urls              jsonb       not null default '[]'
, icons                    jsonb       not null default '{}'
, props                    jsonb       not null default '{}'
, tags                     text[]      not null default '{}'
, modified_at              timestamptz not null default now()
, created_at               timestamptz not null default now()
);

create index if not exists folders_root_id_idx
    on folders(root_id);

create index if not exists folders_parent_id_idx
    on folders(parent_id);

create index if not exists folders_kind_idx
    on folders(kind);

create index if not exists folders_project_id_idx
    on folders(project_id);

create index if not exists folders_tags_idx
    on folders using gin(tags);

comment on table folders is
'Content: discovered filesystem tree. Every entry was found by scanning a watched root.
- kind: parent (depth 1 org folder), folder (depth 2 plain), git (repo), subtree (nested repo)
- path: relative to the watch root
- abs_path: absolute path on disk (unique across all roots)
- props: extensible jsonb — for git/subtree: {role, lang, files, loc, stack, libs, indexed_at, last_error, duplicate_of, label}
- tags: quick-access array; controlled vocabulary in sensei.tags table
- parent_id: null = direct child of watch root; set = child of another folder
- project_id: FK to projects — auto-created 1:1 for git/subtree, user can merge/split';

comment on column folders.id
     is 'Surrogate primary key (UUID).';
comment on column folders.root_id
     is 'Foreign key to folders_to_watch — which watched root this folder was discovered under.';
comment on column folders.parent_id
     is 'Self-referencing FK for folder hierarchy. Null means direct child of the watch root.';
comment on column folders.project_id
     is 'Foreign key to projects — groups this folder into a project. Nullable.';
comment on column folders.kind
     is 'Folder classification: parent (depth 1), folder (depth 2), git (repository), subtree (nested repo).';
comment on column folders.name
     is 'Display name — typically the directory basename.';
comment on column folders.path
     is 'Path relative to the watch root (e.g. "clients/acme/api").';
comment on column folders.abs_path
     is 'Absolute filesystem path for watcher setup and deduplication.';
comment on column folders.remote_urls
     is 'JSON array of git remotes: [{name: "origin", url: "git@..."}]. Empty for non-git folders.';
comment on column folders.icons
     is 'JSON object for display icons: {emoji, devicon, custom}.';
comment on column folders.props
     is 'Extensible JSON metadata. For git/subtree: {role, lang, files, loc, stack:{languages,frameworks,runtimes}, libs, indexed_at, last_error, duplicate_of, label}.';
comment on column folders.tags
     is 'Array of tag strings for quick filtering. Vocabulary controlled by sensei.tags table.';
comment on column folders.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column folders.created_at
     is 'Timestamp when this folder was first discovered.';
