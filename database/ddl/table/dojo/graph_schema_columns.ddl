set search_path to dojo, extensions;

-- The columns of a schema node of a repository's shared graph (#225;
-- graph-sync.md §2), carried on the node in its slice; replaced with it.
create table if not exists graph_schema_columns (
  node_id       uuid        not null references dojo.graph_nodes(id) on delete cascade
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, ordinal       integer
, name          text        not null
, sql_type      text
, primary_key   boolean     not null default false
, not_null      boolean     not null default false
, unique_key    boolean     not null default false
, foreign_key   boolean     not null default false
, enum_typed    boolean     not null default false
, default_expr  text
, note          text
, primary key (node_id, name)
);

comment on table graph_schema_columns is
'Columns of a schema node in a repository''s shared graph. Written only by dojo.file_graph_slices.';
