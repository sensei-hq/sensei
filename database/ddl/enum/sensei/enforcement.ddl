set search_path to sensei, extensions;

-- How much authority a governed rule carries — the override-brake axis,
-- independent of scope. Declared in ascending authority so `ORDER BY
-- enforcement DESC` surfaces the strongest first. `mandatory` is the
-- non-overridable "constitution" tier: a more specific scope may refine a
-- recommended/required rule but can never weaken a mandatory one.
create type enforcement
    as enum ('advisory', 'recommended', 'required', 'mandatory');
