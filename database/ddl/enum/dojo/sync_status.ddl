set search_path to dojo, extensions;

-- Last-known federation health of a membership's connection, surfaced on the
-- Dōjō connections pane. `healthy` = heartbeat within window; `stale` = no
-- heartbeat for > 5m; `error` = connection failing; `authenticating` =
-- mid-pair. Derived on the daemon side and mirrored here for display.
create type dojo.sync_status
    as enum ('healthy', 'stale', 'error', 'authenticating');
