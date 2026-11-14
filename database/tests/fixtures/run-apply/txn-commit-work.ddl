-- A DDL file that manages its own transaction (multi-word COMMIT WORK form).
-- --apply must refuse it (exit 2 with a named message). One statement,
-- creating nothing, so it persists nothing even when the guard is missing (F3).
commit work;
