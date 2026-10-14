set search_path to dojo, extensions;

-- A node of a repository's shared code graph — the daemon's sensei.nodes row,
-- keyed for a plane where its uuid means nothing (#225; graph-sync.md §2–3).
--
-- Identity is (repository, branch, folder_path, fqn): fqn is unique per folder locally,
-- and folder_path is repository-relative. A node with no file (a package, a
-- module, an external placeholder) has a null file_path — the unplaced slice.
--
-- `kind` is text, not the daemon's enum: the vocabulary is the indexer's, and a
-- kind added there must not make the sync fail on this plane.
create table if not exists graph_nodes (
  id            uuid        primary key default gen_random_uuid()
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, branch        text        not null
, file_path     text
, folder_path   text        not null
, fqn           text        not null
, kind          text        not null
, name          text        not null
, parent_fqn    text
, line_start    integer
, line_end      integer
, is_exported   boolean     not null default false
, is_test       boolean     not null default false
, signature     text
, description   text
, docstring     text
, language      text
, community_id  integer
, tags          text[]      not null default '{}'
, props         jsonb       not null default '{}'
, unique (repository_id, branch, folder_path, fqn)
, foreign key (repository_id, branch) references dojo.repository_graph_branches(repository_id, branch) on delete cascade
, foreign key (repository_id, branch, file_path) references dojo.graph_files(repository_id, branch, path) on delete cascade
);

create index if not exists graph_nodes_file on graph_nodes(repository_id, branch, file_path);

comment on table graph_nodes is
'Nodes of a repository''s shared code graph, keyed (repository, branch, folder_path, fqn). A null file_path is the unplaced slice. Written only by dojo.file_graph_slices.';
