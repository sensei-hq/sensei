set search_path to dojo, extensions;

-- The relationship a membership represents, which drives routing and
-- attribution. `client` takes precedence over `employer` when a project could
-- route to either (the client's confidentiality policy governs). `community`
-- is the opt-in global-dojo collective. `personal` is a solo/open-source Dōjō.
create type dojo.membership_kind
    as enum ('employer', 'client', 'community', 'personal');
