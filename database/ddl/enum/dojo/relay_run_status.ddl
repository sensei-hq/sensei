set search_path to dojo, extensions;

-- Cloud-side mirror of a daemon-local run's status, for the Relay phone/console
-- display. The daemon holds the authoritative run_status; this value is published
-- (mirrored) over /v1. `running` = advancing; `paused` = intentional (rate/weekly
-- limit, with a reset time); `stalled` = no progress within the expected window
-- (suspect, auto-diagnosed); `crashed` = unexpected death (distinct from `failed`);
-- `blocked` = waiting on a hard-block gate; `done`/`failed` = terminal.
-- NB: dbd deploys enum variants alphabetically — never rely on declared order; rank
-- with a CASE in queries.
create type dojo.relay_run_status
    as enum ('running', 'paused', 'stalled', 'crashed', 'blocked', 'done', 'failed');
