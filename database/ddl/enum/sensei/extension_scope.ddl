set search_path to sensei, extensions;

create type extension_scope
    as enum ('global', 'project', 'folder');
