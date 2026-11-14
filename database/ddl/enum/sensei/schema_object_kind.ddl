set search_path to sensei, extensions;

-- The database object a schema_entities row describes — the same words a schema
-- node carries in `nodes.declared_type` (spec 20, "The node vocabulary").
-- A closed set: a producer cannot invent a spelling no reader handles. `enum`
-- (2026-11-02): Database · ER's Overview counts a design's enums and opens each
-- on its values, as dbd's viewer does.
create type schema_object_kind
    as enum ('table', 'view', 'materialized_view', 'function', 'procedure', 'enum');
