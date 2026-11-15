-- Add `github` to sensei.tracker_kind on a database whose type predates the GitHub
-- adapter (docs/spec/daemon/tracker-github.md §H7). dbd creates an enum whole on a
-- fresh database but never adds a value to an existing one, so an upgraded install
-- would refuse every GitHub connect as a database error (as `gitlab` did, found
-- live in the GitLab slice's F6, 2026-11-07).
--
-- Appended, as the type's DDL appends it: a value's ordinal is its sort order. Runs
-- after sensei_tracker_kind_gitlab.sql, so the order is jira, gitlab, github.
-- A no-op on a fresh database (no type yet; the DDL creates it with every value)
-- and once the value exists.
do $$
begin
    if to_regtype('sensei.tracker_kind') is not null then
        alter type sensei.tracker_kind add value if not exists 'github';
    end if;
end
$$;
