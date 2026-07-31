set search_path to sensei, extensions;

create table if not exists folder_path_aliases (
  alias_abs_path  text        primary key
, folder_id       uuid        not null references sensei.folders(id) on delete cascade
, reason          text        not null default 'rename'
, created_at      timestamptz not null default now()
);

create index if not exists folder_path_aliases_folder_idx
    on folder_path_aliases(folder_id);

comment on table folder_path_aliases is
'Historical absolute paths a folder previously lived at (before a rename/move).
Project/folder identity is path-keyed on folders.abs_path, so a moved repo would
otherwise orphan its history — every transcript/hook cwd recorded under the old
path stops matching. The path resolvers (get_folder_ids_by_path, find_folder_for_path)
match a cwd against folders.abs_path OR an alias here, so old transcripts still
resolve to the CURRENT folder + project. Populated on remap — an explicit
`folder remap`, or a git-remote-detected rename during reconcile. A folder keeps
its identity (uuid, project, sessions) across a move.';

comment on column folder_path_aliases.alias_abs_path
     is 'A former absolute path of the folder (globally unique — a path can alias at most one folder). Matched by the cwd resolvers.';
comment on column folder_path_aliases.reason
     is 'Why the alias exists: rename (explicit remap) | detected (git-remote reconcile match).';
