set search_path to sensei, extensions;

-- A folder's indexing lifecycle: discovered → queued → indexing → indexed,
-- or failed.
--
-- `archived` = the folder's directory is gone (a deleted or moved repo whose move
-- couldn't be auto-detected) but it has history worth keeping (sessions/transcripts);
-- reconcile RETAINS it instead of hard-deleting, and skips it on the vanish prune.
--
-- `deferred` WAS here, meaning "intentionally not indexed — sibling/standalone".
-- REMOVED: v2 has no such folder. A `folders` row exists only for a repo root
-- or a manifest-bearing directory, and every one of those IS indexed — a
-- directory we do not index simply has no row. The value had no writer
-- anywhere in the tree (only a doc comment and a test fixture named it), so it
-- described a state the system could not reach.
--
-- NB: dbd deploys enum variants alphabetically — never rely on declared order.
create type folder_status
    as enum ('discovered', 'queued', 'indexing', 'indexed', 'failed', 'archived');
