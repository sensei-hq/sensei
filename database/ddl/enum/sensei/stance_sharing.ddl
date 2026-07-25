set search_path to sensei, extensions;

-- The sharing axis of a stance: what surfaces to the dōjō. Feeds federation /
-- contribution — a higher setting publishes more of what a session learns.
-- Declared in ascending openness. `patterns` (share derived patterns, not raw
-- prompts/code) is the default. `derived` = everything derived (still never raw
-- secrets/PII — that is the enforcement plane's job, not this).
create type stance_sharing
    as enum ('private', 'patterns', 'patterns_prompts', 'derived');
