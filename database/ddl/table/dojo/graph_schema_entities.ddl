set search_path to dojo, extensions;

-- What a schema node of a repository's shared graph declares: kind, the database
-- schema it lives in, and where the fact came from (#225; graph-sync.md §2) —
-- the daemon's sensei.schema_entities row, carried on its node in the slice.
--
-- Hangs off its node, so a slice's replace takes it with the node. Text, not the
-- daemon's enums, for the reason graph_nodes.kind is text.
create table if not exists graph_schema_entities (
  node_id       uuid        primary key references dojo.graph_nodes(id) on delete cascade
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, kind          text        not null
, db_schema     text
, source        text
, naming        text
, note          text
, note_md       text
);

comment on table graph_schema_entities is
'The schema entity a shared graph node declares (table, view, …): its database schema and provenance. Written only by dojo.file_graph_slices.';
