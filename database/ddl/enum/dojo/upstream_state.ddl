set search_path to dojo, extensions;

-- State of an item in the upstream contribution queue, as shown on the
-- personal Share-review surface. `queued` = will ship next batch; `held` =
-- withheld, re-queues next batch; `edited` = payload edited by the user, ships
-- with the edit; `sent` = pushed to the Dōjō.
create type dojo.upstream_state
    as enum ('queued', 'held', 'edited', 'sent');
