set search_path to dojo, extensions;

-- The indexes of a schema node of a repository's shared graph (#225;
-- graph-sync.md §2), carried on the node in its slice; replaced with it.
create table if not exists graph_schema_indexes (
  id            bigserial   primary key
, node_id       uuid        not null references dojo.graph_nodes(id) on delete cascade
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, name          text        not null
, definition    text
, unique_index  boolean     not null default false
);

create index if not exists graph_schema_indexes_node on graph_schema_indexes(node_id);

comment on table graph_schema_indexes is
'Indexes of a schema node in a repository''s shared graph. Written only by dojo.file_graph_slices.';
