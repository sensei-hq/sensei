set search_path to sensei, extensions;

create type dml_operation
    as enum ('INSERT', 'UPDATE', 'DELETE');
