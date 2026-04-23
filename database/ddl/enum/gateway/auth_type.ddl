set search_path to gateway, extensions;

create type auth_type
    as enum ('api_key', 'none');
