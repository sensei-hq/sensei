set search_path to sensei, extensions;
create table if not exists folders (
  id                       uuid          primary key default gen_random_uuid()
, root_id                  uuid          not null references sensei.folders_to_watch(id) on delete cascade
, parent_id                uuid          references sensei.folders(id) on delete cascade
, repository_id            uuid          references sensei.repositories(id) on delete set null
, workspace_root_id        uuid          references sensei.folders(id) on delete set null
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
'The scanned filesystem tree — every directory sensei considered worth indexing,
discovered by walking a watched root.

## What a row is

- A directory that is either a repository, a manifest-bearing build unit inside
  one, or an ordinary folder beneath those. A directory sensei does not index
  has NO row here — there is no "not indexed" state to look for.
- Unique on `abs_path`, so one directory is one row no matter how many roots or
  scans reach it.

## Three independent axes, deliberately not collapsed

- **What it IS** — `kind`. A property of the directory alone.
- **Where it is in the LIFECYCLE** — `status`. Changes as indexing progresses;
  says nothing about what the folder is.
- **Who CLAIMS it** — `repository_id` (which repository owns it) and
  `workspace_root_id` (which workspace declares it as a member). Both are
  relationships, so both are stored as FKs rather than folded into `kind`.

Collapsing any two of these is the mistake this shape exists to prevent: a
`kind` that moves when an unrelated manifest gains a `workspaces` array is
describing the wrong thing.

## How to find a project''s folders

A folder does NOT carry a project. That column existed until #211 and was
settable per folder, so a repository''s folders could disagree about their
project — and they did. Membership now lives in
`sensei.project_repositories`, keyed on the REPOSITORY. Resolve through
`sensei.folder_projects`, which is
`folders.repository_id` -> `project_repositories` -> `projects`:

    SELECT folder_id FROM sensei.folder_projects WHERE project = ''sensei''

That resolution is MULTI-VALUED by design: a repository is keyed on its remote,
so one serving two projects yields a row for each.

## Common queries

    -- the repositories under a watch root
    SELECT name, abs_path FROM folders WHERE root_id = $1 AND kind = ''git''
    -- the build units a workspace declares
    SELECT name FROM folders WHERE workspace_root_id = $1
    -- what is still mid-flight
    SELECT abs_path, status FROM folders WHERE status NOT IN (''indexed'', ''archived'')

## Invariants a caller can rely on

- `root_id` is NOT NULL and agrees with `abs_path`: the watch root is always the
  longest `folders_to_watch.path` prefixing it. Checked by the index audit''s
  misrooted-folders class, so it is enforced rather than assumed.
- `repository_id` is carried by EVERY folder, inherited from its repo anchor at
  write time — not only by the repo root. `kind` still marks which folder IS the
  anchor.
- `parent_id` NULL means a direct child of the watch root.';

comment on column folders.id
     is 'Surrogate primary key (UUID).';
comment on column folders.root_id
     is 'Foreign key to folders_to_watch — which watched root this folder was discovered under.';
comment on column folders.parent_id
     is 'Self-referencing FK for folder hierarchy. Null means direct child of the watch root.';
comment on column folders.kind
     is 'What this folder IS: git (repository), module (a manifest-bearing build unit inside a repo — a crate, an npm package, a Go module), subtree (nested git repo), standalone (non-git, no git siblings — written only by v1 scan paths, retiring with them), folder (an ordinary directory inside a repo, no manifest).

It does NOT say who claims the folder. Whether a workspace declares a module is a relationship and lives in workspace_root_id. `workspace_member` and `package` were once two values here; merged, because that split flips for several folders the moment a `workspaces` array is added — nothing about the directory changes — and a kind that moves under an unrelated edit describes the wrong thing.';
comment on column folders.workspace_root_id
     is 'WHICH workspace declares this module as a member — not whether one does.

- Membership is a RELATIONSHIP, so it is stored as one. `kind` says what a
  folder IS; this says who claims it. Naming the workspace rather than a bare
  yes/no makes "the members of this workspace" a single indexed lookup.
- DERIVED from the declaring manifest''s member list (package.json
  `workspaces`, Cargo.toml `[workspace] members`), matched on the repo-relative
  PATH — never on the directory name or its position in the tree. Asserting
  membership no manifest states is the R4 fabrication.
- Refreshed on every scan, so a folder that leaves the member list stops
  reading as a member.
- NULL means nothing declares it. A real and common state, not "unknown":
  `marketplace/`, `app/`, `dojo/` and `website/` here are all genuine build
  units that no workspace lists.
- KNOWN GAP, named rather than silent: only the REPO ROOT''s manifest is read
  today, so a nested workspace root is not detected and its members resolve as
  undeclared.';
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
     is 'FK to sensei.repositories — the global repository this folder belongs to.

SET ON EVERY FOLDER, inherited from its repo anchor at WRITE time by
upsert_subfolder_kind / upsert_folder. `kind` (git/standalone/subtree) is what
marks which folder IS the anchor.

SUPERSEDES I16 (2026-10-01, see docs/decisions.md D-REPO-ANCHOR), which set this
only on the anchor and had subfolders resolve via repo_anchor_for. That is a
set-returning FUNCTION, and a predicate cannot be pushed into a function scan —
`WHERE project = $1` seq-scanned all 13,715 folders with 13,715 function calls to
answer a 45-row question, and four of eight views migrated onto it regressed
catastrophically (one ~450x). Resolving the anchor once per folder WRITTEN
instead of per row READ makes every read a Bitmap Index Scan on
folders_repository_id_idx.

NULL only while the scanner has not yet resolved the owning repository.
ON DELETE SET NULL.';
comment on column folders.branch
     is 'The checked-out branch for this checkout folder (develop vs main = two folders, one repository). Metrics-only seam — does NOT make the code graph branch-aware.';
comment on column folders.modified_at
     is 'Timestamp of the last modification to this row.';
