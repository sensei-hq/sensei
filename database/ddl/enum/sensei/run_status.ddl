set search_path to sensei, extensions;

-- Authoritative status of a daemon-local autonomous run (activity.runs). The cloud
-- mirror is dojo.relay_run_status. `running` = advancing; `paused` = intentional
-- (rate/weekly limit, with paused_until); `stalled` = no progress within the
-- expected window (auto-diagnosed); `crashed` = unexpected death (distinct from
-- `failed` = terminal work verdict); `blocked` = waiting on a hard-block gate;
-- `done`/`failed` = terminal. dbd orders variants alphabetically — rank via CASE.
create type run_status
    as enum ('running', 'paused', 'stalled', 'crashed', 'blocked', 'done', 'failed');
