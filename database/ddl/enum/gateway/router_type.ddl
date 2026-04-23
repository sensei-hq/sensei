set search_path to gateway, extensions;

create type router_type
    as enum ('direct', 'aggregator', 'local');
