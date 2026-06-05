set search_path to sensei, extensions;

create table if not exists folder_namespaces (
  folder_id                uuid        not null references sensei.folders(id) on delete cascade
, namespace_id             uuid        not null references sensei.namespaces(id) on delete cascade
, modified_at              timestamptz not null default now()
, primary key (folder_id, namespace_id)
);

create index if not exists folder_namespaces_namespace_id_idx
    on folder_namespaces(namespace_id);

comment on table folder_namespaces is
'Set membership: which namespaces a repo (folder) belongs to. A repo typically
joins one namespace per applicable scope — organization, project, team, plus a
technology namespace per detected language. Resolution gathers rules/identity
across every member namespace + the implicit user/general scopes, ordered by
scope level. Populated at scan time from README frontmatter + detected stack.';

comment on column folder_namespaces.folder_id
     is 'FK to sensei.folders — the repo.';
comment on column folder_namespaces.namespace_id
     is 'FK to sensei.namespaces — a scope instance the repo belongs to.';
comment on column folder_namespaces.modified_at
     is 'Timestamp of the last modification to this row.';
