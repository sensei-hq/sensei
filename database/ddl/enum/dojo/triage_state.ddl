set search_path to dojo, extensions;

-- Position of a candidate in the maintainer triage queue. `queued` = waiting;
-- `in_review` = a maintainer opened it; `auto_approved` = cleared the automated
-- trust bar (collective, e.g. >= 5 contributors at confidence >= 0.8);
-- `resolved` = a human decision was recorded (see dojo.decisions);
-- `merged` = folded into a near-duplicate existing artifact.
create type dojo.triage_state
    as enum ('queued', 'in_review', 'auto_approved', 'resolved', 'merged');
