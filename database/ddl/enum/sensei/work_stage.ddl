set search_path to sensei, extensions;

-- Which stage of the work a session was doing, for locating rework and
-- bottlenecks (spec 2026-08-26-thematic-retrospectives).
--
-- "Rework is high" is not actionable; "rework concentrates in build while plan
-- depth scores 2" says the plan was thin. That inference needs a stage per
-- session, and activity.runs.current_phase covers only sessions run through a
-- playbook — 10 of 151 on the daemon today.
create type work_stage
    as enum ('explore', 'analyze', 'plan', 'build', 'verify', 'fix', 'operate');
