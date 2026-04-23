set search_path to sensei, extensions;

create type task_type_kind
    as enum ('feat', 'fix', 'refactor', 'docs', 'test', 'chore', 'unknown');
