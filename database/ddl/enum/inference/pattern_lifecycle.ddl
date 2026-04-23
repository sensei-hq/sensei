set search_path to inference, extensions;

create type pattern_lifecycle
    as enum ('suggested', 'gap', 'rule');
