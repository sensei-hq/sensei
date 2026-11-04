-- A DDL file that manages its own transaction. --apply must refuse it (exit 2
-- with a named message): an inner commit/rollback would end the wrapper early
-- and autocommit the tail.
create or replace function dojo.__apply_txn_marker() returns int language sql as $$ select 1 $$;
commit;
