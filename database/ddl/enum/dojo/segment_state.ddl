set search_path to dojo, extensions;

-- State of a Relay segment (a Phase or, nested, a Step). `pending` = not started;
-- `active` = in progress; `done` = shipped; `skipped` = deliberately not done;
-- `failed` = terminal failure; `blocked` = waiting on a hard-block gate;
-- `needs_review` = complete but awaiting the human's async review.
create type dojo.segment_state
    as enum ('pending', 'active', 'done', 'skipped', 'failed', 'blocked', 'needs_review');
