set search_path to sensei, extensions;

create type folder_status
    as enum ('discovered', 'queued', 'indexing', 'indexed', 'failed', 'deferred');
