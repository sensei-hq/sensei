set search_path to sensei, extensions;

create type service_kind
    as enum ('data', 'api', 'devtool', 'service', 'inference');
