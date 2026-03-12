-- sensei seed data loader
-- Called by dbd import after staging tables are populated
-- Executes import procedures in dependency order

-- Phase 1: Independent catalog tables
call staging.import_repos();
call staging.import_libraries();

-- Phase 2: Event data (references repo_id from phase 1)
call staging.import_events();
call staging.import_benchmark_reports();
