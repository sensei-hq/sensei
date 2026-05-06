set search_path to sensei, extensions;

create type task_status
    as enum ('in_progress', 'completed', 'abandoned');
