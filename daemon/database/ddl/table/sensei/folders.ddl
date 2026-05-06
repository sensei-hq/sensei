set search_path to sensei, extensions;
create table if not exists folders (
  id                       uuid          primary key default gen_random_uuid()
, root_id                  uuid          not null references sensei.folders_to_watch(id) on delete cascade
, parent_id                uuid          references sensei.folders(id) on delete cascade
, project_id               uuid          references sensei.projects(id) on delete set null
, kind                     folder_kind   not null default 'git'
, status                   folder_status not null default 'discovered'
, role                     folder_role
, name                     text          not null
, path                     text          not null
, abs_path                 text          not null unique
, stack                    jsonb         not null default '[]'
, remote_urls              jsonb         not null default '[]'
, icons                    jsonb         not null default '{}'
, props                    jsonb         not null default '{}'
, tags                     text[]        not null default '{}'
, modified_at              timestamptz   not null default now()
);

create index if not exists folders_root_id_idx
    on folders(root_id);

create index if not exists folders_parent_id_idx
    on folders(parent_id);

create index if not exists folders_kind_idx
    on folders(kind);

create index if not exists folders_status_idx
    on folders(status);

create index if not exists folders_project_id_idx
    on folders(project_id);

create index if not exists folders_tags_idx
    on folders using gin(tags);

comment on table folders is
'Content: discovered filesystem tree. Every entry was found by scanning a watched root.
- kind: git (repository), workspace_member (monorepo member), subtree (nested git repo), sibling (non-git sibling of git folders), standalone (non-git, no git siblings)
- status: discovered (found), queued (files counted), indexing (in progress), indexed (complete), failed, deferred (not indexed — sibling/standalone)
- stack: detected technology stack ["rust", "typescript", "svelte"] — set by ProcessGitFolder
- path: relative to the watch root
- abs_path: absolute path on disk (unique across all roots)
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
     is 'Folder classification: git (repository), workspace_member (monorepo member), subtree (nested git repo), sibling (non-git sibling), standalone (non-git, no git siblings).';
comment on column folders.status
     is 'Indexing lifecycle: discovered → queued → indexing → indexed. Or deferred (not indexed — sibling/standalone). Or failed.';
comment on column folders.stack
     is 'Detected technology stack as JSON array: ["rust", "typescript"]. Set by ProcessGitFolder from config files (Cargo.toml, package.json, etc.).';
comment on column folders.role
     is 'Project role assigned during setup: backend, frontend, library, docs, infra. Null until assigned.';
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
