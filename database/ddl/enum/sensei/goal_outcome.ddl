set search_path to sensei, extensions;

-- Whether a session achieved what the person set out to do, as READ FROM THE
-- TRANSCRIPT by the facet analyzer (spec 2026-08-26).
--
-- Deliberately NOT sensei.session_outcome, which answers a different question.
-- session_outcome is mechanical — `corrected` means corrections were counted,
-- `empty`/`incomplete` describe capture state. This is a judgment about the
-- GOAL, and its values (`mostly_achieved`, `unclear`) have no mechanical
-- equivalent. Folding them into one column would force a lossy mapping and make
-- "corrected" and "mostly_achieved" look like alternatives to each other when a
-- session is routinely both.
create type goal_outcome
    as enum ('completed', 'mostly_achieved', 'partial', 'blocked', 'abandoned', 'unclear');
