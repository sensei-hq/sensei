set search_path to sensei, extensions;

-- Is this state fine, decided, or broken?
--
-- The axis is "does this clear itself WITHOUT a human?" — not "did anyone
-- decide?". That distinction is what lets one registry serve every domain: a
-- schedule that is not due yet and an org that has not enabled sharing are both
-- "didn't happen", and only one of them needs anybody's attention.
--
-- Generalises a call this project already made once: sensei.sync_state has
-- `skipped` alongside `error` precisely because "a private repository is not a
-- sync failure, it is a choice".
create type reason_kind as enum ('normal', 'refusal', 'fault');

comment on type reason_kind is
'normal = nothing is wrong and it resolves itself (no remedy, no actor).
refusal = somebody deliberately decided; name who, and what to do.
fault   = something that should have happened did not; needs attention.

The axis is self-resolution, NOT whether a decision was made. A code carrying a
remedy is never `normal` — a CHECK on sensei.reason_codes enforces that, because
rendering a stuck state as unalarmed plain text is how "not syncing" becomes
indistinguishable from "nothing to sync".';
