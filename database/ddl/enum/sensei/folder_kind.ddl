set search_path to sensei, extensions;

create type folder_kind
    as enum ('parent', 'folder', 'git', 'subtree');
