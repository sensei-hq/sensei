set search_path to sensei, extensions;

create type extension_source
    as enum ('builtin', 'marketplace', 'local');
