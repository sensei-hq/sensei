set search_path to sensei, extensions;

-- `archived` = the folder's directory is gone (a deleted or moved repo whose move
-- couldn't be auto-detected) but it has history worth keeping (sessions/transcripts);
-- reconcile RETAINS it instead of hard-deleting, and skips it on the vanish prune.
-- NB: dbd deploys enum variants alphabetically — never rely on declared order.
create type folder_status
    as enum ('discovered', 'queued', 'indexing', 'indexed', 'failed', 'deferred', 'archived');
