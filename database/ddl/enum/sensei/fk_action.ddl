set search_path to sensei, extensions;

-- A foreign key's ON DELETE action, in dbd SchemaModel v2's words.
-- NULL on a schema_refs row means the source did not state one.
create type fk_action
    as enum ('cascade', 'restrict', 'set_null', 'set_default', 'no_action');
