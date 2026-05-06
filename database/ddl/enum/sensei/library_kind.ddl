set search_path to sensei, extensions;

create type library_kind
    as enum ('detected', 'imported');
