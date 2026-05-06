set search_path to sensei, extensions;

create type folder_kind
    as enum ('git', 'workspace_member', 'subtree', 'sibling', 'standalone');
