set search_path to sensei, extensions;

create table if not exists schema_entities (
  node_id     uuid               primary key references sensei.nodes(id) on delete cascade
, kind        schema_object_kind not null
, db_schema   text               not null
, source      schema_source      not null
, naming      schema_naming      not null
, note        text
, note_md     text
, modified_at timestamptz        not null default now()
);

comment on table schema_entities is
'One database object — a table, view, materialized view, function or procedure —
described in detail. The object itself is a NODE (`nodes.id`), which is what puts
it in the code graph; this row and its children carry what the graph does not:
the schema it lives in, who described it, and (in schema_columns /
schema_indexes / schema_refs) its columns, indexes and column-level foreign keys.

- **Why a separate store.** Columns as nodes would multiply the SQL half of the
  graph ~10x and force an embedding decision per column; detail in
  `nodes.props` would bloat every node read to serve one screen. Decision
  2026-10-08, docs/plan/decisions.md; spec docs/spec/indexer/20-schema-entities.md.
- **Who writes it.** `persist::write_schema`, from the dbd importer (a
  `SchemaBatch`) and from ORM extractors (`FileFacts.schema`), in the same
  transaction as the node. A re-index REPLACES an object''s detail.
- **Lifetime.** Cascades from the node: an object removed by a re-index takes
  its columns, indexes and refs with it.';
