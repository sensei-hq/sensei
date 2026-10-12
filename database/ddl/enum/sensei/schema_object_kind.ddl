set search_path to sensei, extensions;

-- The database object a schema_entities row describes — the same five words a
-- schema node carries in `nodes.declared_type` (spec 20, "The node vocabulary").
-- A closed set: a producer cannot invent a sixth spelling no reader handles.
create type schema_object_kind
    as enum ('table', 'view', 'materialized_view', 'function', 'procedure');
