set search_path to sensei, extensions;

create type snapshot_kind
    as enum ('manual', 'checkpoint');
