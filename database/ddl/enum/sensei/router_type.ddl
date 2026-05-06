set search_path to sensei, extensions;

create type router_type
    as enum ('direct', 'aggregator', 'local');
