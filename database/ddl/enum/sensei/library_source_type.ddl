set search_path to sensei, extensions;

create type library_source_type
    as enum ('llms.txt', 'http', 'local');
