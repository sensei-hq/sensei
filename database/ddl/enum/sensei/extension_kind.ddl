set search_path to sensei, extensions;

create type extension_kind
    as enum ('plugin', 'skill', 'command', 'agent', 'hook');
