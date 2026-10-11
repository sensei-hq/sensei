set search_path to sensei, extensions;

create table if not exists schema_indexes (
  id           bigserial primary key
, node_id      uuid      not null references sensei.schema_entities(node_id) on delete cascade
, name         text
, definition   text      not null
, unique_index boolean   not null default false
);

create index if not exists schema_indexes_node_id_idx
    on schema_indexes(node_id);

comment on table schema_indexes is
'The indexes of a schema object: dbd SchemaModel v2''s `Index {def, unique,
name}`. `definition` is the column spec as written — `(status, placed_at DESC)`
— an expression, so text and not a structured column list. `name` may be NULL:
dbd reports unnamed indexes. Replaced wholesale when the object is re-indexed.';
