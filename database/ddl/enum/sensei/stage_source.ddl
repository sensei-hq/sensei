set search_path to sensei, extensions;

-- Whether a session's stage was DECLARED by the developer or inferred from the
-- transcript by the analyzer.
--
-- Not decoration. "38% of rework happens in build" means something different if
-- a model guessed the stage than if the developer said so, and any surface that
-- shows a stage rollup has to be able to tell the reader which it is. Keeping
-- the two in one column would make an inference indistinguishable from a fact.
create type stage_source
    as enum ('recorded', 'inferred');
