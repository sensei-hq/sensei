-- Add `not_private` to sensei.tracker_writeback_status on a database whose type predates
-- comment write-back on GitLab and GitHub (docs/spec/daemon/tracker-forge-comment-writeback.md
-- F7). dbd creates an enum whole on a fresh database but never adds a value to an
-- existing one.
--
-- Appended, as the type's DDL appends it: a value's ordinal is its sort order.
-- A no-op on a fresh database (no type yet; the DDL creates it with every value)
-- and once the value exists.
do $$
begin
    if to_regtype('sensei.tracker_writeback_status') is not null then
        alter type sensei.tracker_writeback_status add value if not exists 'not_private';
    end if;
end
$$;
