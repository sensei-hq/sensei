set search_path to sensei, extensions;

create type memory_scope
    as enum ('global', 'workspace', 'stack', 'task_type', 'module');
