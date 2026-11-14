-- A DDL file that manages its own transaction. --apply must refuse it (exit 2
-- with a named message): an inner commit/rollback would end the wrapper early
-- and autocommit the tail. One statement, creating nothing, so it persists
-- nothing even when the guard is missing (F3).
commit;
