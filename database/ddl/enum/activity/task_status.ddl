set search_path to activity, extensions;

create type task_status
    as enum ('in_progress', 'completed', 'abandoned');
