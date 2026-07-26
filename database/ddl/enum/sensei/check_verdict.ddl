set search_path to sensei, extensions;

-- The outcome of running a rule's checker (D-CHECKER). `pass`/`fail` = the
-- checker ran and returned a clean / non-zero exit; `skipped` = the rule is
-- checker-backed but no runnable command resolved for this repo (e.g. no `lint`
-- command discovered); `error` = the checker was launched but timed out or
-- could not be spawned. Declared alphabetically (dbd sorts enum variants
-- alphabetically anyway — code maps by string, never by ordinal).
create type check_verdict as enum ('error', 'fail', 'pass', 'skipped');
