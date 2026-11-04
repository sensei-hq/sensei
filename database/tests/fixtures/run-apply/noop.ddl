-- A DDL file that persists nothing even if committed: it pairs with
-- commits.probe, so the test-file guard case is safe while that guard is
-- missing (F5).
select 1;
