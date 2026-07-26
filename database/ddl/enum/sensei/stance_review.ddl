set search_path to sensei, extensions;

-- The review axis of a stance: who must sign off before a rule adopts. Feeds the
-- adoption flow — a higher setting requires more approvers before a proposed rule
-- becomes governing. Declared in ascending strictness. `one_maintainer` is the
-- default; `me_alone` is the solo case; `quorum` requires a maintainer quorum.
create type stance_review
    as enum ('me_alone', 'one_maintainer', 'two_maintainers', 'quorum');
