set search_path to sensei, extensions;

-- Collapse a task-execution error message to its SHAPE, so failures can be
-- grouped by cause instead of by the path that happened to trigger them.
--
-- ## Why this exists
--
-- Error messages embed the file they are about, so grouping on the raw text
-- gives one group per file and a count of 1 — useless for triage. Measured
-- 2026-09-23: the stage-3 barrier failure produced 17,577,049 rows over 43,312
-- distinct paths. Raw grouping says "43,312 problems"; the signature says
-- "1 problem, 43,312 times", which is the sentence that leads to a fix.
--
-- Two substitutions, in order:
--   1. anything parenthesised -> `(…)`. Our error messages carry the subject in
--      parens — `process_file fatal DB write (crates/…/walk.rs)`.
--   2. anything path-shaped (a token containing `/`) -> `…`. Catches the paths
--      that appear outside parens, e.g. `no files row for docs/spec/x.md`.
--
-- It deliberately does NOT strip fqns or identifiers. `upsert module node
-- rust·senseid·foo` and `… typescript·dojo·bar` stay distinct, because the
-- language and package ARE the grouping a reader wants for a duplicate-identity
-- failure. Only the filesystem path is noise.
--
-- IMMUTABLE so it can be grouped on and indexed: the mapping depends on nothing
-- but its input.
create or replace function error_signature(p_message text)
returns text
language sql
immutable
parallel safe
set search_path = sensei, extensions
as $$
  select regexp_replace(
           regexp_replace(p_message, '\([^)]*\)', '(…)', 'g'),
           '[A-Za-z0-9._@-]*/[A-Za-z0-9._/@-]+', '…', 'g')
$$;

comment on function error_signature(text) is
'Collapse an error message to its shape for GROUP BY: parenthesised subjects ->
(…), path-shaped tokens -> …. Language/package/fqn tokens are KEPT, because a
duplicate-identity failure in rust and one in typescript are different problems.
Turns "43,312 errors" into "1 error, 43,312 times".';
