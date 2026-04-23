set search_path to inference, extensions;

create type pattern_severity
    as enum ('low', 'medium', 'high');
