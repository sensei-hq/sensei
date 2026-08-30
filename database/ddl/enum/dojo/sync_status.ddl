set search_path to dojo, extensions;

-- Last-known federation health of a membership's connection, surfaced on the
-- Dōjō connections pane. `healthy` = heartbeat within window; `stale` = no
-- heartbeat for > 5m; `error` = connection failing; `authenticating` =
-- mid-pair.
--
-- INTENDED to be derived on the daemon side and mirrored here for display. That
-- derivation DOES NOT EXIST: nothing on either side of the fork writes a value
-- other than the `authenticating` literal set at pairing. Two consumers read as
-- if it did — admin-data.ts computes `error_rate_1h` as
-- count(sync_status = 'error'), which is therefore structurally pinned at 0 and
-- cannot report a sync error even during one; and the app's connections pane
-- counts `sync_status === 'healthy'`, which is permanently 0. The type is kept
-- because the vocabulary is right; only the producer is missing.
create type dojo.sync_status
    as enum ('healthy', 'stale', 'error', 'authenticating');
