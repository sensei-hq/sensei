set search_path to sensei, extensions;

create table if not exists schema_columns (
  node_id      uuid    not null references sensei.schema_entities(node_id) on delete cascade
, ordinal      integer not null
, name         text    not null
, sql_type     text    not null
, primary_key  boolean not null default false
, not_null     boolean not null default false
, unique_key   boolean not null default false
, foreign_key  boolean not null default false
, enum_typed   boolean not null default false
, default_expr text
, note         text
, primary key (node_id, name)
);

comment on table schema_columns is
'The columns of a schema object, in declaration order. dbd SchemaModel v2''s
`Column {name, ty, pk, nn, uq, fk, en, def, note}`, renamed only where a SQL
keyword collides: `sql_type` is the declared type VERBATIM, `default_expr` the
default expression as written. `foreign_key` is the flag a renderer needs
without scanning schema_refs; WHICH key it is lives there. Unique per
(object, name). Replaced wholesale when the object is re-indexed.';
