set search_path to sensei, extensions;

create type pattern_lifecycle
    as enum ('suggested', 'gap', 'rule');
