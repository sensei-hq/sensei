set search_path to sensei, extensions;

create type auth_type
    as enum ('api_key', 'none');
