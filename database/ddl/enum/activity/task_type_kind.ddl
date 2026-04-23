set search_path to activity, extensions;

create type task_type_kind
    as enum ('feat', 'fix', 'refactor', 'docs', 'test', 'chore', 'unknown');
