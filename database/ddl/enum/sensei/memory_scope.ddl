set search_path to sensei, extensions;

create type memory_scope
    as enum ('global', 'project', 'stack', 'task_type', 'module');
