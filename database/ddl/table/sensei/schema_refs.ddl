set search_path to sensei, extensions;

create table if not exists schema_refs (
  id           bigserial primary key
, from_node_id uuid      not null references sensei.schema_entities(node_id) on delete cascade
, from_column  text      not null
, to_node_id   uuid      not null references sensei.schema_entities(node_id) on delete cascade
, to_column    text      not null
, on_delete    fk_action
);

create index if not exists schema_refs_from_node_id_idx
    on schema_refs(from_node_id);

create index if not exists schema_refs_to_node_id_idx
    on schema_refs(to_node_id);

comment on table schema_refs is
'Column-level foreign keys: one row per column PAIR, which is how dbd
SchemaModel v2 carries a composite key (`Ref {from {s,t,c}, to {s,t,c},
action}`). The TABLE-level relationship is also a `references` edge in
sensei.edges, so the graph answers "what touches this table"; this row is the
detail an ER diagram draws. Cascades from BOTH ends: removing either object
removes the line rather than leaving one to nothing. `on_delete` NULL means
the source stated no action.';
