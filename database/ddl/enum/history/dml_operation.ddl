set search_path to history, extensions;

create type dml_operation
    as enum ('INSERT', 'UPDATE', 'DELETE');
