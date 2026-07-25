set search_path to dojo, extensions;

-- How conformance to a rule is decided — the verifiability axis that turns a rule
-- from documentation into something the daemon can point at (or run). `manual` =
-- human judgement, the default; `checker` = an automated static check (lint rule,
-- regex, or command) named in `checker_ref`; `test` = satisfied by a named test;
-- `review` = a human reviewer signs off at a gate. Orthogonal to enforcement:
-- a mandatory rule may still be `manual`, an advisory rule may have a `checker`.
create type dojo.rule_check
    as enum ('manual', 'checker', 'test', 'review');
