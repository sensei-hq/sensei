-- Add `gitlab` to sensei.tracker_kind on a database whose type predates the GitLab
-- adapter (docs/spec/daemon/tracker-gitlab.md §G10). dbd creates an enum whole on a
-- fresh database but never adds a value to an existing one, so an upgraded install
-- would keep `jira` alone and refuse every GitLab connect as a database error
-- (found live, F6, 2026-11-07).
--
-- Appended, as the type's DDL appends it: a value's ordinal is its sort order.
-- A no-op on a fresh database (no type yet; the DDL creates it with both values)
-- and once the value exists.
do $$
begin
    if to_regtype('sensei.tracker_kind') is not null then
        alter type sensei.tracker_kind add value if not exists 'gitlab';
    end if;
end
$$;
