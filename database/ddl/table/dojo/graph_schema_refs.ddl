set search_path to dojo, extensions;

-- A foreign key of a schema node of a repository's shared graph (#225;
-- graph-sync.md §2). The referencing side is a row; the referenced table is a
-- KEY (to_folder, to_fqn), resolved against graph_nodes of the same branch when
-- read — the rule graph_edges' targets follow, so a key into a file not yet
-- filed is kept and resolves when it arrives.
create table if not exists graph_schema_refs (
  id            bigserial   primary key
, from_node_id  uuid        not null references dojo.graph_nodes(id) on delete cascade
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, from_column   text
, to_folder     text
, to_fqn        text
, to_column     text
, on_delete     text
);

create index if not exists graph_schema_refs_from on graph_schema_refs(from_node_id);

comment on table graph_schema_refs is
'Foreign keys of a schema node in a repository''s shared graph: from a row, to a (folder, fqn) key resolved at read time. Written only by dojo.file_graph_slices.';
