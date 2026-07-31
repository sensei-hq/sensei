set search_path to dojo, extensions;

-- The project's adoption phase in the loop: `watch` (observing, no rules pushed
-- yet) → `notice` (surfacing candidate rules) → `adopt` (governance in force).
-- The order is semantic (watch < notice < adopt), but dbd deploys enum variants
-- ALPHABETICALLY (adopt, notice, watch) — never rely on declared order; rank with
-- a CASE in queries / a lookup map in code.
create type dojo.project_phase
    as enum ('watch', 'notice', 'adopt');
