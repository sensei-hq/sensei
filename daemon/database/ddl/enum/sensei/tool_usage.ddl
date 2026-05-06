set search_path to sensei, extensions;

create type tool_usage
    as enum ('used', 'partial', 'ignored');
