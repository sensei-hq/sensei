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
, repository_id            uuid          references sensei.repositories(id) on delete set null
  -- WHICH workspace declares this module, when one does.
  --
  -- Membership is a RELATIONSHIP, so it is stored as one — not as a `kind`.
  -- `kind` says what a folder IS; this says who claims it, and says WHICH
  -- workspace rather than a bare yes/no, so "the members of this workspace"
  -- is a single indexed lookup.
  --
  -- NULL means nothing declares it: a real and common state. `marketplace/`,
  -- `app/`, `dojo/` and `website/` here are all real build units that no
  -- workspace lists. NULL is "undeclared", never "unknown".
  --
  -- DERIVED from the declaring manifest's member list (package.json
  -- `workspaces`, Cargo.toml `[workspace] members`), matched on the
  -- repo-relative PATH. Refreshed on every scan, because a folder that leaves
  -- the list must stop reading as a member.
, workspace_root_id        uuid          references sensei.folders(id) on delete set null
, branch                   text
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

-- Covering index for the repository_id FK (nullable): a repository delete cascades
-- to SET NULL here, and the repo-grain compute joins folders → repository.
create index if not exists folders_repository_id_idx
    on folders(repository_id) where repository_id is not null;

-- "the members of this workspace" — the reason membership is a relationship
-- rather than an enum value.
create index if not exists folders_workspace_root_id_idx
    on folders(workspace_root_id) where workspace_root_id is not null;

comment on table folders is
'Content: discovered filesystem tree. Every entry was found by scanning a watched root.
- kind: git (repository), module (a manifest-bearing build unit inside a repo), subtree (nested git repo), sibling (non-git sibling of git folders), standalone (non-git, no git siblings), folder (ordinary directory, no manifest)
- workspace_root_id: which workspace declares this module; null = declared by nothing
- status: discovered (found), queued (files counted), indexing (in progress), indexed (complete), failed, archived (directory gone, history kept)
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
     is 'What this folder IS: git (repository), module (a manifest-bearing build unit inside a repo — a crate, an npm package, a Go module), subtree (nested git repo), sibling (non-git sibling), standalone (non-git, no git siblings), folder (an ordinary directory inside a repo, no manifest).

It does NOT say who claims the folder. Whether a workspace declares a module is a relationship and lives in workspace_root_id. `workspace_member` and `package` were once two values here; merged, because that split flips for several folders the moment a `workspaces` array is added — nothing about the directory changes — and a kind that moves under an unrelated edit describes the wrong thing.';
comment on column folders.workspace_root_id
     is 'The folder whose manifest DECLARES this module as a workspace member (package.json `workspaces`, Cargo.toml `[workspace] members`). NULL means nothing declares it — a real state, and the common one for a repo with several independent build units. Derived from the declaring manifest and matched on repo-relative PATH, never on the directory name or its position in the tree; asserting membership no manifest states is the R4 fabrication. Refreshed on every scan so a folder that leaves the member list stops reading as a member.

Only the REPO ROOT''s manifest is read today, so a nested workspace root is not detected — its members resolve as undeclared. A named gap, not a silent one.';
comment on column folders.status
     is 'Indexing lifecycle: discovered → queued → indexing → indexed, or failed. Or archived, when the directory is gone but its history is worth keeping.

There is no "not indexed" state. A folders row exists only for a repo root or a manifest-bearing directory, and every one of those is indexed; a directory we do not index has no row at all.';
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
comment on column folders.repository_id
     is 'FK to sensei.repositories — the global repository this checkout belongs to. Set ONLY on the repo-root/checkout folder (I16); subfolders resolve via nearest ancestor (repo_anchor_for). NULL until the scanner resolves the remote. ON DELETE SET NULL.';
comment on column folders.branch
     is 'The checked-out branch for this checkout folder (develop vs main = two folders, one repository). Metrics-only seam — does NOT make the code graph branch-aware.';
comment on column folders.modified_at
     is 'Timestamp of the last modification to this row.';
